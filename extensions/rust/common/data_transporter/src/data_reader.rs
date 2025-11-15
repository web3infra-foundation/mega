use model::tugraph_model::{
    Application, ApplicationVersion, Library, LibraryVersion, Program, UProgram, UVersion,
};
use semver::Version;
use serde_json::Value;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    time::Instant,
};
use tokio_postgres::NoTls;
use tudriver::tugraph_client::TuGraphClient;

use crate::{
    db::{db_connection_config_from_env, db_cratesio_connection_config_from_env, DBHandler},
    handler::{
        Crateinfo, DependencyCount, DependencyCrateInfo, DependencyInfo, DependentCount,
        DependentData, DependentInfo, Deptree, Versionpage,
    },
    NameVersion,
};
#[allow(async_fn_in_trait)]
pub trait DataReaderTrait {
    async fn get_all_programs_id(&self) -> Vec<String>;
    async fn get_program(&self, program_id: &str) -> Result<Program, Box<dyn Error>>;
    async fn get_type(&self, program_id: &str) -> Result<(UProgram, bool), Box<dyn Error>>;
    async fn get_versions(
        &self,
        program_id: &str,
        is_lib: bool,
    ) -> Result<Vec<crate::VersionInfo>, Box<dyn Error>>;
    async fn get_direct_dependency_nodes(
        &self,
        name_and_version: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>>;
    async fn new_get_direct_dependency_nodes(
        &self,
        namespace: &str,
        nameversion: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn get_program_by_name(&self, program_name: &str)
        -> Result<Vec<Program>, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn get_indirect_dependency_nodes(
        &self,
        nameversion: NameVersion,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn count_dependencies(&self, nameversion: NameVersion) -> Result<usize, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn get_direct_dependent_nodes(
        &self,
        name_and_version: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>>;
    async fn new_get_direct_dependent_nodes(
        &self,
        namespace: &str,
        nameversion: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn get_indirect_dependent_nodes(
        &self,
        nameversion: NameVersion,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>>;

    //async fn get_max_version(&self, name: String) -> Result<String, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn get_lib_version(&self, name: String) -> Result<Vec<String>, Box<dyn Error>>;
    async fn new_get_lib_version(
        &self,
        namespace: String,
        name: String,
    ) -> Result<Vec<String>, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn get_app_version(&self, name: String) -> Result<Vec<String>, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn new_get_app_version(
        &self,
        namespace: String,
        name: String,
    ) -> Result<Vec<String>, Box<dyn Error>>;
    
    #[allow(dead_code)]
    async fn get_all_dependencies(
        &self,
        nameversion: NameVersion,
    ) -> Result<HashSet<String>, Box<dyn Error>>;
    async fn new_get_all_dependencies(
        &self,
        nodes: Vec<NameVersion>,
    ) -> Result<HashSet<String>, Box<dyn Error>>;
    
    #[allow(dead_code)]
    async fn new_get_all_dependents(
        &self,
        namespace: String,
        nameversion: String,
    ) -> Result<HashSet<String>, Box<dyn Error>>;
    async fn get_github_url(
        &self,
        namespace: String,
        name: String,
    ) -> Result<String, Box<dyn Error>>;
    async fn get_doc_url(&self, namespace: String, name: String) -> Result<String, Box<dyn Error>>;
    async fn build_graph(
        &self,
        rootnode: &mut Deptree,
        visited: &mut HashSet<String>,
    ) -> Result<(), Box<dyn Error>>;
    async fn get_version_page_from_tg(
        &self,
        nsfront: String,
        nsbehind: String,
        nname: String,
    ) -> Result<Vec<Versionpage>, Box<dyn Error>>;
    async fn get_crates_front_info_from_tg(
        &self,
        nname: String,
        nversion: String,
        nsfront: String,
        nsbehind: String,
    ) -> Result<Crateinfo, Box<dyn Error>>;
    async fn get_dependency_from_tg(
        &self,
        name: String,
        version: String,
        nsfront: String,
        nsbehind: String,
    ) -> Result<DependencyInfo, Box<dyn Error>>;
    async fn get_dependent_from_tg(
        &self,
        name: String,
        version: String,
        nsfront: String,
        nsbehind: String,
    ) -> Result<DependentInfo, Box<dyn Error>>;
}

#[derive(Clone)]
pub struct DataReader {
    pub client: TuGraphClient,
}
impl DataReader {
    pub async fn new(
        uri: &str,
        user: &str,
        password: &str,
        db: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let client = TuGraphClient::new(uri, user, password, db).await?;
        Ok(DataReader { client })
    }
}

impl DataReaderTrait for DataReader {
    async fn get_dependent_from_tg(
        &self,
        name: String,
        version: String,
        nsfront: String,
        nsbehind: String,
    ) -> Result<DependentInfo, Box<dyn Error>> {
        let namespace = nsfront.clone() + "/" + &nsbehind.clone();
        let nameversion = name.clone() + "/" + &version.clone();
        let mut direct_nodes = vec![];
        if version.clone() == *"all" {
            let lib_versions = self
                .new_get_lib_version(namespace.clone(), name.clone())
                .await
                .unwrap();
            let mut getversions = vec![];
            for version in lib_versions {
                getversions.push(version);
            }
            getversions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);

                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a), // 从高到低排序
                    (Ok(_), Err(_)) => Ordering::Less,   // 无法解析的版本号认为更小
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            let mut visited = HashSet::new();
            for nversion in &getversions {
                let tmp_name_and_version = name.clone() + "/" + nversion;
                let tmp_direct_nodes = self
                    .new_get_direct_dependent_nodes(&namespace, &tmp_name_and_version)
                    .await
                    .unwrap();
                for node in tmp_direct_nodes {
                    if visited.insert(node.clone()) {
                        direct_nodes.push(node.clone());
                    }
                }
            }
        } else {
            direct_nodes = self
                .new_get_direct_dependent_nodes(&namespace, &nameversion)
                .await
                .unwrap();
        }
        let getdirect_count = direct_nodes.len();
        
        let mut deps = vec![];
        let mut count1 = 0;
        for item in direct_nodes {
            let dep = DependentData {
                crate_name: item.clone().name,
                version: item.clone().version,
                relation: "Direct".to_string(),
            };
            deps.push(dep);
            count1 += 1;
            if count1 == 50 {
                break;
            }
        }
        

        let res_deps = DependentInfo {
            direct_count: getdirect_count,
            indirect_count: 0,
            data: deps,
        };
        Ok(res_deps)
    }
    async fn get_dependency_from_tg(
        &self,
        name: String,
        version: String,
        nsfront: String,
        nsbehind: String,
    ) -> Result<DependencyInfo, Box<dyn Error>> {
        let namespace = nsfront.clone() + "/" + &nsbehind.clone();
        let nameversion = name.clone() + "/" + &version.clone();
        tracing::info!("{} {}", namespace.clone(), nameversion.clone());
        let mut direct_nodes = vec![];
        if version.clone() == *"all" {
            let lib_versions = self
                .new_get_lib_version(namespace.clone(), name.clone())
                .await
                .unwrap();
            let mut getversions = vec![];
            for version in lib_versions {
                getversions.push(version);
            }
            getversions.sort_by(|a, b| {
                let version_a = Version::parse(a);
                let version_b = Version::parse(b);

                match (version_a, version_b) {
                    (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a), // 从高到低排序
                    (Ok(_), Err(_)) => Ordering::Less,   // 无法解析的版本号认为更小
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                }
            });
            let mut visited = HashSet::new();
            for nversion in &getversions {
                let tmp_name_and_version = name.clone() + "/" + nversion;
                let tmp_direct_nodes = self
                    .new_get_direct_dependency_nodes(&namespace, &tmp_name_and_version)
                    .await
                    .unwrap();
                for node in tmp_direct_nodes {
                    if visited.insert(node.clone()) {
                        direct_nodes.push(node.clone());
                    }
                }
            }
        } else {
            direct_nodes = self
                .new_get_direct_dependency_nodes(&namespace, &nameversion)
                .await
                .unwrap();
        }
        let getdirect_count = direct_nodes.len();
        let all_dependency_nodes = self
            .new_get_all_dependencies(direct_nodes.clone())
            .await
            .unwrap();
        let mut indirect_dependency = vec![];
        for node in all_dependency_nodes {
            let mut dr = false;
            for node2 in direct_nodes.clone() {
                let nv = node2.name.clone() + "/" + &node2.version.clone();
                if node == nv {
                    dr = true;
                    break;
                }
            }
            if !dr {
                indirect_dependency.push(node);
            }
        }
        let db_connection_config = db_connection_config_from_env();
        #[allow(unused_variables)]
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
            .await
            .unwrap();
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });
        let dbhandler = DBHandler { client };
        let indirect_dependency_count = indirect_dependency.len();
        let mut deps = vec![];
        for item in direct_nodes {
            let dep_count = dbhandler
                .get_all_dependency_counts_from_pg(item.clone().name, item.clone().version)
                .await
                .unwrap();
            let dep = DependencyCrateInfo {
                crate_name: item.clone().name,
                version: item.clone().version,
                relation: "Direct".to_string(),
                license: "".to_string(),
                dependencies: dep_count,
            };
            deps.push(dep);
        }
        for item in indirect_dependency {
            let parts: Vec<&str> = item.split('/').collect();
            let newitem = NameVersion {
                name: parts[0].to_string(),
                version: parts[1].to_string(),
            };
            let dep_count = dbhandler
                .get_all_dependency_counts_from_pg(newitem.clone().name, newitem.clone().version)
                .await
                .unwrap();

            let dep = DependencyCrateInfo {
                crate_name: parts[0].to_string(),
                version: parts[1].to_string(),
                relation: "Indirect".to_string(),
                license: "".to_string(),
                dependencies: dep_count,
            };
            deps.push(dep);
        }

        let res_deps = DependencyInfo {
            direct_count: getdirect_count,
            indirect_count: indirect_dependency_count,
            data: deps,
        };
        Ok(res_deps)
    }
    #[allow(unused_assignments)]
     async fn get_crates_front_info_from_tg(
        &self,
        nname: String,
        nversion: String,
        nsfront: String,
        nsbehind: String,
    ) -> Result<Crateinfo, Box<dyn Error>> {
        let db_connection_config = db_connection_config_from_env();
        #[allow(unused_variables)]
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
            .await
            .unwrap();
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });
        let dbhandler = DBHandler { client };
        let namespace = nsfront.clone() + "/" + &nsbehind.clone();
        let name_and_version = nname.clone() + "/" + &nversion.clone();
        let mut githuburl = self
            .get_github_url(namespace.clone(), nname.clone())
            .await
            .unwrap();
        if githuburl == *"null" || githuburl == *"None" {
            githuburl = "".to_string();
        }
        let mut docurl = self
            .get_doc_url(namespace.clone(), nname.clone())
            .await
            .unwrap();
        if docurl == *"null" || docurl == *"None" {
            docurl = "".to_string();
        }
        let lib_versions = self
            .new_get_lib_version(namespace.clone(), nname.clone())
            .await
            .unwrap();
        let mut getversions = vec![];
        for version in lib_versions {
            getversions.push(version);
        }
        getversions.sort_by(|a, b| {
            let version_a = Version::parse(a);
            let version_b = Version::parse(b);

            match (version_a, version_b) {
                (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a), // 从高到低排序
                (Ok(_), Err(_)) => Ordering::Less,   // 无法解析的版本号认为更小
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => Ordering::Equal,
            }
        });
        let mut direct_dependency_nodes = vec![];
        let mut direct_dependent_nodes = vec![];
        let mut getcves = vec![];
        let mut get_dependency_cves = vec![];
        let mut direct_dependency_count = 0;
        let mut indirect_dependency_count = 0;
        let mut direct_dependent_count = 0;
        if nversion.clone() == *"all" {
            let (direct_depends, indirect_depends, direct_depended_by, depends_cves_id) = dbhandler
                .get_depends_of_all_version(nname.clone())
                .await
                .unwrap();
            direct_dependency_count = direct_depends;
            indirect_dependency_count = indirect_depends;
            direct_dependent_count = direct_depended_by;
            let mut visited = HashSet::new();
            for version in &getversions {
                let tmp_direct_nodes = dbhandler.get_direct_rustsec(&nname, version).await.unwrap();
                for node in tmp_direct_nodes {
                    if visited.insert(node.clone()) {
                        getcves.push(node.clone());
                    }
                }
            }
            get_dependency_cves = dbhandler
                .get_rustsec_by_depends_cveid(depends_cves_id.clone())
                .await
                .unwrap();
        } else {
            direct_dependency_nodes = self
                .new_get_direct_dependency_nodes(&namespace, &name_and_version)
                .await
                .unwrap();
            direct_dependency_count = direct_dependency_nodes.len();
            let all_dependency_nodes = self
                .new_get_all_dependencies(direct_dependency_nodes.clone())
                .await
                .unwrap();
            let mut indirect_dependency = vec![];
            for node in all_dependency_nodes.clone() {
                let mut dr = false;
                for node2 in direct_dependency_nodes.clone() {
                    let nv = node2.name.clone() + "/" + &node2.version.clone();
                    if node == nv {
                        dr = true;
                        break;
                    }
                }
                if !dr {
                    indirect_dependency.push(node);
                }
            }
            indirect_dependency_count = indirect_dependency.len();
            direct_dependent_nodes = self
                .new_get_direct_dependent_nodes(&namespace, &name_and_version)
                .await
                .unwrap();
            direct_dependent_count = direct_dependent_nodes.len();
            getcves = dbhandler
                .get_direct_rustsec(&nname, &nversion)
                .await
                .unwrap();
            get_dependency_cves = dbhandler
                .get_dependency_rustsec(all_dependency_nodes.clone())
                .await
                .unwrap();
        }

        let getlicense = dbhandler
            .get_license_by_name(&namespace, &nname)
            .await
            .unwrap();
        let dcy_count = DependencyCount {
            direct: direct_dependency_count,
            indirect: indirect_dependency_count,
        };
        let dt_count = DependentCount {
            direct: direct_dependent_count,
            indirect: 0,
        };
        let res = Crateinfo {
            crate_name: nname.clone(),
            description: "".to_string(),
            dependencies: dcy_count,
            dependents: dt_count,
            cves: getcves,
            versions: getversions,
            license: getlicense[0].clone(),
            github_url: githuburl,
            doc_url: docurl,
            dep_cves: get_dependency_cves,
        };
        Ok(res)
    }
    #[allow(unused_assignments)]
    async fn get_version_page_from_tg(
        &self,
        nsfront: String,
        nsbehind: String,
        nname: String,
    ) -> Result<Vec<Versionpage>, Box<dyn Error>> {
        let namespace = nsfront.clone() + "/" + &nsbehind;
        let time_get_version = Instant::now();
        let all_versions = self
            .new_get_lib_version(namespace.clone(), nname.clone())
            .await
            .unwrap();
        tracing::info!("finish get all versions");
        let mut getversions = vec![];
        for version in all_versions {
            getversions.push(version);
        }
        getversions.sort_by(|a, b| {
            let version_a = Version::parse(a);
            let version_b = Version::parse(b);

            match (version_a, version_b) {
                (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => Ordering::Equal,
            }
        });
        tracing::info!("get_version_time:{:?}", time_get_version.elapsed());
        let mut every_version = vec![];
        let db_connection_config = db_connection_config_from_env();
        #[allow(unused_variables)]
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
            .await
            .unwrap();
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });
        tracing::info!("finish connect pg");
        let dbhandler = DBHandler { client };
        let _db_cratesio_connection_config = db_cratesio_connection_config_from_env();
        #[allow(unused_variables)]
        let (client2, connection2) =
            tokio_postgres::connect(&_db_cratesio_connection_config, NoTls)
                .await
                .unwrap();
        tokio::spawn(async move {
            if let Err(e) = connection2.await {
                eprintln!("connection error: {e}");
            }
        });
        let dbhandler2 = DBHandler { client: client2 };
        for version in getversions {
            
            let all_dts = dbhandler
                .get_all_dependent_counts_from_pg(nname.clone(), version.clone())
                .await
                .unwrap();
            let res = dbhandler2
                .get_dump_from_cratesio_pg(nname.clone(), version.clone())
                .await
                .unwrap();
            tracing::info!("finish get dump from pg");
            if !res.is_empty() {
                let parts: Vec<&str> = res.split("/").collect();
                if parts.len() == 2 {
                    let versionpage = Versionpage {
                        version,
                        updated_at: parts[0].to_string(),
                        downloads: parts[1].to_string(),
                        dependents: all_dts,
                    };
                    every_version.push(versionpage);
                }
            }
        }
        Ok(every_version)
    }
    async fn build_graph(
        &self,
        rootnode: &mut Deptree,
        visited: &mut HashSet<String>,
    ) -> Result<(), Box<dyn Error>> {
        let name_and_version = &rootnode.name_and_version;
        let res = self
            .get_direct_dependency_nodes(name_and_version)
            .await
            .unwrap();
        tracing::info!("direct dep count:{}", res.len());
        let db_connection_config = db_connection_config_from_env();
        #[allow(unused_variables)]
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
            .await
            .unwrap();
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });
        let dbhandler = DBHandler { client };
        for node in res {
            let name = node.name.clone();
            let version = node.version.clone();
            let dep_nv = name.clone() + "/" + &version;
            let rustcve = dbhandler.get_direct_rustsec(&name, &version).await.unwrap();
            let mut dn = Deptree {
                name_and_version: dep_nv.clone(),
                cve_count: rustcve.len(),
                direct_dependency: Vec::new(),
            };
            if visited.insert(dep_nv.clone()) {
                Box::pin(self.build_graph(&mut dn, visited)).await?;
                rootnode.direct_dependency.push(dn);
            }
        }
        Ok(())
    }
    async fn get_github_url(
        &self,
        namespace: String,
        name: String,
    ) -> Result<String, Box<dyn Error>> {
        tracing::info!("{}|{}", namespace.clone(), name.clone());
        let query = format!(
            "
            MATCH (n:program {{namespace:'{namespace}'}}) WHERE n.name='{name}'
RETURN n.github_url 
        "
        );
        let results = self.client.exec_query(&query).await?;
        tracing::info!("finish get github_url");
        let mut res = vec![];
        for node in results {
            res.push(node);
        }
        let unique_items: HashSet<String> = res.clone().into_iter().collect();
        let mut nodes = vec![];
        for res in unique_items {
            let parsed: Value = serde_json::from_str(&res).unwrap();
            if let Some(url) = parsed.get("n.github_url").and_then(|v| v.as_str()) {
                nodes.push(url.to_string());
            }
        }
        nodes.push("None".to_string());
        Ok(nodes[0].clone())
    }
    async fn get_doc_url(&self, namespace: String, name: String) -> Result<String, Box<dyn Error>> {
        let query = format!(
            "
            MATCH (n:program {{namespace:'{namespace}'}}) WHERE n.name='{name}'
RETURN n.doc_url 
        "
        );
        let results = self.client.exec_query(&query).await?;
        let mut res = vec![];
        for node in results {
            res.push(node);
        }
        let unique_items: HashSet<String> = res.clone().into_iter().collect();
        let mut nodes = vec![];
        for res in unique_items {
            let parsed: Value = serde_json::from_str(&res).unwrap();
            if let Some(url) = parsed.get("n.doc_url").and_then(|v| v.as_str()) {
                nodes.push(url.to_string());
            }
        }
        nodes.push("None".to_string());
        Ok(nodes[0].clone())
    }
    async fn get_all_dependencies(
        &self,
        nameversion: NameVersion,
    ) -> Result<HashSet<String>, Box<dyn Error>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let name_and_version = nameversion.name.clone() + "/" + &nameversion.version.clone();
        queue.push_back(name_and_version.to_string());
        let mut count = 0;
        while let Some(current) = queue.pop_front() {
            count += 1;
            if count == 2000 {
                break;
            }
            if visited.insert(current.clone()) {
                for dep in self.get_direct_dependency_nodes(&current).await.unwrap() {
                    let tmp = dep.name.clone() + "/" + &dep.version.clone();
                    queue.push_back(tmp);
                }
            }
        }
        visited.remove(&name_and_version);

        Ok(visited)
    }
    async fn new_get_all_dependencies(
        &self,
        nodes: Vec<NameVersion>,
    ) -> Result<HashSet<String>, Box<dyn Error>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        for node in nodes {
            let nameandversion = node.clone().name + "/" + &node.clone().version;
            queue.push_back(nameandversion.clone());
        }
        let mut count = 0;
        while let Some(current) = queue.pop_front() {
            count += 1;
            if count == 2000 {
                break;
            }
            if visited.insert(current.clone()) {
                for dep in self.get_direct_dependency_nodes(&current).await.unwrap() {
                    let tmp = dep.name.clone() + "/" + &dep.version.clone();
                    queue.push_back(tmp);
                }
            }
        }
        Ok(visited)
    }
    
    async fn new_get_all_dependents(
        &self,
        namespace: String,
        nameversion: String,
    ) -> Result<HashSet<String>, Box<dyn Error>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        for node in self
            .new_get_direct_dependent_nodes(&namespace, &nameversion)
            .await
            .unwrap()
        {
            let nameandversion = node.clone().name + "/" + &node.clone().version;
            queue.push_back(nameandversion.clone());
        }
        let len = queue.len();
        if len < 500 {
            while let Some(current) = queue.pop_front() {
                if visited.insert(current.clone()) {
                    for dep in self.get_direct_dependent_nodes(&current).await.unwrap() {
                        let tmp = dep.name.clone() + "/" + &dep.version.clone();
                        queue.push_back(tmp);
                    }
                }
            }
        }
        Ok(visited)
    }
    async fn get_all_programs_id(&self) -> Vec<String> {
        tracing::info!("start get all ids");
        let query = "
            MATCH (p: program)
            RETURN p 
        ";

        let results = self.client.exec_query(query).await.unwrap();

        let mut programs = vec![];
        for result in results {
            let programs_json: Value = serde_json::from_str(&result).unwrap();

            let pro = programs_json["p"].clone();

            let program: Program = serde_json::from_value(pro).unwrap();

            programs.push(program.id);
        }
    tracing::info!("finish get all ids");
        programs
    }

    async fn get_program(&self, program_id: &str) -> Result<Program, Box<dyn Error>> {
        let query = format!(
            "
            MATCH (p: program {{id: '{program_id}'}})
            RETURN p            
            "
        );
        let results = self.client.exec_query(&query).await?;
        let programs_json: Value = serde_json::from_str(&results[0]).unwrap();
        let pro = programs_json["p"].clone();
        let program: Program = serde_json::from_value(pro).unwrap();
        Ok(program)
    }

    async fn get_type(&self, program_id: &str) -> Result<(UProgram, bool), Box<dyn Error>> {
        let mut islib = false;

        let query = format!(
            "
            MATCH (p: program {{id: '{program_id}'}})-[:has_type]->(o)
            RETURN o, label(o) as o_label
            "
        );

        let results = self.client.exec_query(&query).await?;
        let mut uprograms = vec![];
        for result in results {
            let result_json: Value = serde_json::from_str(&result).unwrap();

            let label: String = serde_json::from_value(result_json["o_label"].clone()).unwrap();

            let o = result_json["o"].clone();
            if label.eq(&"library".to_string()) {
                islib = true;
                let library: Library = serde_json::from_value(o).unwrap();
                uprograms.push(UProgram::Library(library));
            } else if label.eq(&"application".to_string()) {
                let application: Application = serde_json::from_value(o).unwrap();
                uprograms.push(UProgram::Application(application));
            }
        }
        Ok((uprograms[0].clone(), islib))
    }

    async fn get_versions(
        &self,
        program_id: &str,
        is_lib: bool,
    ) -> Result<Vec<crate::VersionInfo>, Box<dyn Error>> {
        let query = if is_lib {
            format!(
                "
                MATCH (l: library {{id: '{program_id}'}})-[:has_version]->(o)
                RETURN o
            "
            )
        } else {
            format!(
                "
                MATCH (l: application {{id: '{program_id}'}})-[:has_version]->(o)
                RETURN o
                "
            )
        };

        let results = self.client.exec_query(&query).await?;

        let mut versions: Vec<crate::VersionInfo> = vec![];
        for result in results {
            let result_json: Value = serde_json::from_str(&result).unwrap();

            let o = result_json["o"].clone();

            let (version_base, name_version) = if is_lib {
                let library_version: LibraryVersion = serde_json::from_value(o).unwrap();
                (
                    UVersion::LibraryVersion(library_version.clone()),
                    library_version.name_and_version.clone(),
                )
            } else {
                let application_version: ApplicationVersion = serde_json::from_value(o).unwrap();
                (
                    UVersion::ApplicationVersion(application_version.clone()),
                    application_version.name_and_version.clone(),
                )
            };
            tracing::debug!("Read version for id {program_id}: {:?}", version_base);

            // get dependencies
            let dependencies = self
                .get_direct_dependency_nodes(&name_version)
                .await
                .unwrap();

            versions.push(crate::VersionInfo {
                version_base,
                dependencies,
            })
        }
        Ok(versions)
    }

    async fn get_direct_dependency_nodes(
        &self,
        name_and_version: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>> {
        let query = format!(
            "
                MATCH (n:version {{name_and_version: '{name_and_version}'}})-[:depends_on]->(m:version)
                RETURN m.name_and_version as name_and_version
                "
        );

        let results = self.client.exec_query(&query).await?;
        let unique_items: HashSet<String> = results.clone().into_iter().collect();
        let mut nodes = vec![];

        for result in unique_items {
            let result_json: Value = serde_json::from_str(&result).unwrap();
            let name_version_str: String =
                serde_json::from_value(result_json["name_and_version"].clone()).unwrap();

            if let Some(name_version) = crate::NameVersion::from_string(&name_version_str) {
                nodes.push(name_version);
            }
        }

        Ok(nodes)
    }
    async fn new_get_direct_dependency_nodes(
        &self,
        namespace: &str,
        nameversion: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>> {
        tracing::info!("enter get_direct_dependency_nodes");
        let query1 = format!(
            "
                MATCH (p:program {{namespace: '{namespace}'}})-[:has_type]->(l)-[:has_version]->(lv {{name_and_version: '{nameversion}'}})-[:has_dep_version]->(vs:version)-[:depends_on]->(m:version)
RETURN m.name_and_version as name_and_version
                "
        );
        let results1 = self.client.exec_query(&query1).await?;
        tracing::info!("finish get_direct_dep");
        let mut res = vec![];
        for node in results1 {
            res.push(node);
        }
        let unique_items: HashSet<String> = res.clone().into_iter().collect();
        let mut nodes = vec![];

        for result in unique_items {
            let result_json: Value = serde_json::from_str(&result).unwrap();
            let name_version_str: String =
                serde_json::from_value(result_json["name_and_version"].clone()).unwrap();

            if let Some(name_version) = crate::NameVersion::from_string(&name_version_str) {
                nodes.push(name_version);
            }
        }
        let mut real_nodes = vec![];
        if !nodes.is_empty(){
            let mut version_map: HashMap<String, Vec<String>> = HashMap::new();
            for node in nodes.clone(){
                version_map.entry(node.name)
                .or_default()
                .push(node.version);
            }
            for (name,versions) in version_map.clone(){
                let mut sorting_versions: Vec<semver::Version> = versions
                .iter()
                .filter_map(|ver| semver::Version::parse(ver).ok()) // 将所有版本字符串解析为 Version 对象
                .collect();
                sorting_versions.sort();
                if let Some(real_deps_version) = sorting_versions.first(){
                    let real_deps_version = real_deps_version.to_string();
                    let res_nv = NameVersion{ name, version: real_deps_version };
                    real_nodes.push(res_nv);
                }
            }
        }
        Ok(real_nodes)
    }
    async fn get_indirect_dependency_nodes(
        &self,
        nameversion: NameVersion,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>> {
        let name_and_version = nameversion.name + "/" + &nameversion.version;
        let mut nodes = self
            .get_direct_dependency_nodes(&name_and_version)
            .await
            .unwrap();
        for node in nodes.clone() {
            tracing::info!("{} {}", node.clone().name, node.clone().version);
            let new_nodes = Box::pin(self.get_indirect_dependency_nodes(node))
                .await
                .unwrap();
            for new_node in new_nodes {
                tracing::info!("{} {}", new_node.clone().name, new_node.clone().version);
                nodes.push(new_node);
            }
        }
        Ok(nodes)
    }
    async fn get_program_by_name(
        &self,
        program_name: &str,
    ) -> Result<Vec<Program>, Box<dyn Error>> {
        let query = format!(
            "
            MATCH (p:program)
            WHERE p.name CONTAINS '{program_name}'
            RETURN p
            "
        );
        let results = self.client.exec_query(&query).await?;
        let mut programs = vec![];
        for result in results {
            let programs_json: Value = serde_json::from_str(&result).unwrap();
            let pro = programs_json["p"].clone();
            let program: Program = serde_json::from_value(pro).unwrap();
            programs.push(program);
        }
        Ok(programs)
    }
    async fn count_dependencies(&self, nameversion: NameVersion) -> Result<usize, Box<dyn Error>> {
        let all_nodes = self.get_all_dependencies(nameversion).await.unwrap();
        Ok(all_nodes.len())
    }
    async fn get_direct_dependent_nodes(
        &self,
        name_and_version: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>> {
        let query = format!(
            "
                MATCH (n:version {{name_and_version: '{name_and_version}'}})<-[:depends_on]-(m:version)
                RETURN m.name_and_version as name_and_version
                "
        );

        let results = self.client.exec_query(&query).await?;
        let unique_items: HashSet<String> = results.clone().into_iter().collect();
        let mut nodes = vec![];

        for result in unique_items {
            let result_json: Value = serde_json::from_str(&result).unwrap();
            let name_version_str: String =
                serde_json::from_value(result_json["name_and_version"].clone()).unwrap();

            if let Some(name_version) = crate::NameVersion::from_string(&name_version_str) {
                nodes.push(name_version);
            }
        }

        Ok(nodes)
    }
    async fn new_get_direct_dependent_nodes(
        &self,
        namespace: &str,
        nameversion: &str,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>> {
        let _ = namespace;
        let query1 = format!(
            "
                MATCH (m:version)-[r:depends_on]->(vs:version{{name_and_version:'{}'}})
                RETURN m.name_and_version as name_and_version 
                ",
            nameversion
        );
        let results1 = self.client.exec_query(&query1).await?;
        let mut res = vec![];
        for node in results1 {
            res.push(node);
        }
        let unique_items: HashSet<String> = res.clone().into_iter().collect();

        let mut nodes = vec![];

        for result in unique_items {
            let result_json: Value = serde_json::from_str(&result).unwrap();
            let name_version_str: String =
                serde_json::from_value(result_json["name_and_version"].clone()).unwrap();

            if let Some(name_version) = crate::NameVersion::from_string(&name_version_str) {
                nodes.push(name_version);
            }
        }

        Ok(nodes)
    }
    async fn get_indirect_dependent_nodes(
        &self,
        nameversion: NameVersion,
    ) -> Result<Vec<crate::NameVersion>, Box<dyn Error>> {
        let name_and_version = nameversion.name + "/" + &nameversion.version;
        let mut nodes = self
            .get_direct_dependent_nodes(&name_and_version)
            .await
            .unwrap();
        for node in nodes.clone() {
            let new_nodes = Box::pin(self.get_indirect_dependent_nodes(node))
                .await
                .unwrap();
            for new_node in new_nodes {
                nodes.push(new_node);
            }
        }
        Ok(nodes)
    }

    async fn get_lib_version(&self, name: String) -> Result<Vec<String>, Box<dyn Error>> {
        let query = format!(
            "
            MATCH (n:library_version {{name: '{name}'}}) RETURN n.version LIMIT 100"
        );

        let results = self.client.exec_query(&query).await.unwrap();

        let mut realres = vec![];

        for res in results {
            let parsed: Value = serde_json::from_str(&res).unwrap();
            if let Some(version) = parsed.get("n.version").and_then(|v| v.as_str()) {
                realres.push(version.to_string());
            }
        }

        Ok(realres)
    }
    async fn new_get_lib_version(
        &self,
        namespace: String,
        name: String,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let _ = namespace;
        let query = format!(
            "
            MATCH (n:library_version {{name:'{}'}}) 
            RETURN n.version as version
            UNION 
            MATCH (m:application_version {{name:'{}'}}) 
            RETURN m.version as version",
            name.clone(),
            name.clone(),
        );
        let time1 = Instant::now();
        let results = self.client.exec_query(&query).await.unwrap();
        let query_time = time1.elapsed();
        tracing::info!("query_statement_need_time:{:?}", query_time);
        let unique_items: HashSet<String> = results.clone().into_iter().collect();

        let mut realres = vec![];

        for res in unique_items {
            let parsed: Value = serde_json::from_str(&res).unwrap();
            if let Some(version) = parsed.get("version").and_then(|v| v.as_str()) {
                realres.push(version.to_string());
            }
        }

        Ok(realres)
    }
    async fn get_app_version(&self, name: String) -> Result<Vec<String>, Box<dyn Error>> {
        let query = format!(
            "
            MATCH (n:application_version {{name: '{name}'}}) RETURN n.version LIMIT 100"
        );
        let results = self.client.exec_query(&query).await.unwrap();
        let mut realres = vec![];
        for res in results {
            let parsed: Value = serde_json::from_str(&res).unwrap();
            if let Some(version) = parsed.get("n.version").and_then(|v| v.as_str()) {
                realres.push(version.to_string());
            }
        }
        Ok(realres)
    }
    async fn new_get_app_version(
        &self,
        namespace: String,
        name: String,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let query = format!(
            "
            MATCH (p:program {{namespace: '{namespace}'}})-[:has_type]->(a:application)-[:has_version]->(av:application_version {{name:'{name}'}})
RETURN av.version"
            
        );

        let results = self.client.exec_query(&query).await.unwrap();
        let unique_items: HashSet<String> = results.clone().into_iter().collect();

        let mut realres = vec![];

        for res in unique_items {
            let parsed: Value = serde_json::from_str(&res).unwrap();
            if let Some(version) = parsed.get("av.version").and_then(|v| v.as_str()) {
                realres.push(version.to_string());
            }
        }

        Ok(realres)
    }
}
