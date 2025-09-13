use crate::git::get_all_git_tags_with_time_sorted;
use crate::utils::name_join_version;
use crate::ImportContext;
use git2::{Oid, Repository};
use git2::{TreeWalkMode, TreeWalkResult};
use model::tugraph_model::DependsOn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use toml::Value;

/// A representation for the info
/// extracted from a `cargo.toml` file
#[allow(unused)]
#[derive(Debug, Default, Clone)]
pub struct Dependencies {
    pub(crate) crate_name: String,
    pub(crate) version: String,
    pub(crate) dependencies: Vec<(String, String)>,

    pub(crate) git_url: String,
    pub(crate) tag_name: String,
}

impl ImportContext {
    /// a git repo contains different crates
    #[allow(clippy::type_complexity)]
    pub(crate) async fn parse_all_versions_of_a_repo(
        &self,
        repo_path: &PathBuf,
        git_url: &str,
    ) -> Vec<Dependencies> {
        let mut crate_version_map: HashMap<(String, String), Dependencies> = HashMap::default();

        let versions = get_all_git_tags_with_time_sorted(repo_path).await; //tag id time

        // parse each version of a repository with an order of time, walk all the packages of it
        for (tag_name, tree, _) in versions.iter() {
            let all_packages_dependencies = self
                .parse_a_repo_of_a_version(repo_path, git_url, tag_name, *tree)
                .await;

            // NOTE: At certain time, a version in Cargo.toml will exists in several tags,
            //  while a tag corresponds to a unique Cargo.toml version.
            //  So, I use a map to select the lastest tag which contains the version.
            for dependencies in all_packages_dependencies {
                let name = dependencies.crate_name.clone();
                let version = dependencies.version.clone();
                crate_version_map.insert((name.clone(), version.clone()), dependencies);
            }
        }

        crate_version_map.into_values().collect()
    }

    /// for a given commit(version), walk all the package
    async fn parse_a_repo_of_a_version(
        &self,
        repo_path: &PathBuf,
        git_url: &str,
        tag_name: &str,
        tree: Oid,
    ) -> Vec<Dependencies> {
        let mut res = Vec::new();

        // Lock the repository and tree for reading
        let repo = Repository::open(repo_path).unwrap();
        let tree = repo.find_tree(tree).expect("Failed to find tree");

        // Walk the tree to find Cargo.toml
        tree.walk(TreeWalkMode::PostOrder, |_, entry| {
            if entry.name() == Some("Cargo.toml") {
                // for each Cargo.toml in repo of given commit
                let obj = entry
                    .to_object(&repo)
                    .expect("Failed to convert TreeEntry to Object");
                let blob = obj.as_blob().expect("Failed to interpret object as blob");
                let content = std::str::from_utf8(blob.content())
                    .expect("Cargo.toml content is not valid UTF-8");

                if let Some(dependencies) =
                    self.parse_a_package_of_a_version(content, git_url, tag_name)
                {
                    res.push(dependencies);
                }
            }

            TreeWalkResult::Ok
        })
        .unwrap();

        res
    }

    fn parse_a_package_of_a_version(
        &self,
        cargo_toml_content: &str,
        git_url: &str,
        tag_name: &str,
    ) -> Option<Dependencies> {
        match cargo_toml_content.parse::<Value>() {
            Ok(toml) => {
                if let Some(package) = toml.get("package") {
                    if let Some(crate_name) = package.get("name") {
                        let crate_name = crate_name.as_str()?.to_string();
                        let version = package.get("version")?.as_str()?.to_string();

                        // e.g. 0.1.53a2 is invalid version number.
                        if semver::Version::parse(&version).is_err() {
                            return None;
                        }

                        // dedup
                        if self
                            .version_updater
                            .version_parser
                            .exists(&crate_name, &version)
                        {
                            return None;
                        }

                        let mut dependencies = vec![];

                        if let Some(dep_table) = toml.get("dependencies") {
                            if let Some(deps_table) = dep_table.as_table() {
                                for (name, val) in deps_table {
                                    if let Some(version) = val.as_str() {
                                        dependencies.push((name.clone(), version.to_owned()));
                                    } else if let Some(ver_tab) = val.as_table() {
                                        if let Some(val) = ver_tab.get("version") {
                                            if let Some(version) = val.as_str() {
                                                dependencies
                                                    .push((name.clone(), version.to_owned()));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let dependencies = Dependencies {
                            crate_name,
                            version,
                            dependencies,
                            git_url: git_url.to_string(),
                            tag_name: tag_name.to_string(),
                        };

                        return Some(dependencies);
                    }
                }
            }
            Err(_) => tracing::error!("Failed to parse Cargo.toml for {:?}", cargo_toml_content),
        }
        None
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VersionUpdater {
    /// a reverse record: who depends on the key?
    pub reverse_depends_on_map: HashMap<String, Vec<(String, model::general_model::Version)>>,

    /// a actual map: a crate **actually** depends on which?
    /// it is used to build `depends_on` edges.
    pub actually_depends_on_map:
        HashMap<model::general_model::Version, Vec<model::general_model::Version>>,

    pub version_parser: VersionParser,
}

impl VersionUpdater {
    pub async fn to_depends_on_edges(&self) -> Vec<DependsOn> {
        let mut edges = vec![];
        for (src, dsts) in &self.actually_depends_on_map {
            for dst in dsts {
                #[allow(non_snake_case)]
                let SRC_ID = name_join_version(&src.name, &src.version);

                #[allow(non_snake_case)]
                let DST_ID = name_join_version(&dst.name, &dst.version);
                let depends_on = DependsOn { SRC_ID, DST_ID };
                edges.push(depends_on);
            }
        }
        edges
    }

    /// Given a dependency list,
    pub async fn update_depends_on(&mut self, info: &Dependencies) {
        self.version_parser
            .insert_version(&info.crate_name, &info.version)
            .await;
        let cur_release = model::general_model::Version::new(&info.crate_name, &info.version);
        self.ensure_dependencies(&cur_release, info).await;
        self.ensure_dependents(&cur_release).await;
    }

    async fn ensure_dependencies(
        &mut self,
        cur_release: &model::general_model::Version,
        info: &Dependencies,
    ) {
        for (name, version) in &info.dependencies {
            //let dep = model::general_model::Version::new(&name, &version);
            self.insert_reverse_dep(name, version, &cur_release.name, &cur_release.version)
                .await;
        }

        // a new version should not exist before.
        assert!(!self.actually_depends_on_map.contains_key(cur_release));
        let cur_dependencies = self.search_dependencies(info).await;
        self.actually_depends_on_map
            .insert(cur_release.clone(), cur_dependencies);
    }

    async fn search_dependencies(&self, info: &Dependencies) -> Vec<model::general_model::Version> {
        let mut res: Vec<model::general_model::Version> = vec![];
        for (dependency_name, dependency_version) in &info.dependencies {
            let version_option = self
                .version_parser
                .find_latest_matching_version(dependency_name, dependency_version)
                .await;

            if let Some(dependency_actual_version) = &version_option {
                let dependency =
                    model::general_model::Version::new(dependency_name, dependency_actual_version);
                res.push(dependency);
            }
        }
        res
    }

    async fn ensure_dependents(&mut self, cur_release: &model::general_model::Version) {
        let sem_ver = semver::Version::parse(&cur_release.version)
            .unwrap_or_else(|_| panic!("failed to parse version {:?}", &cur_release));
        let wrapped_reverse_map = self.reverse_depends_on_map.get(&cur_release.name);
        if let Some(reverse_map) = wrapped_reverse_map {
            for (required_version, reverse_dep) in reverse_map {
                let requirement = match semver::VersionReq::parse(required_version) {
                    Ok(req) => req,
                    Err(_) => {
                        tracing::error!("failed to transform to VersionReq");
                        continue;
                    }
                };

                if requirement.matches(&sem_ver) {
                    if let Some(v) = self.actually_depends_on_map.get_mut(reverse_dep) {
                        let mut found = false;
                        let mut exist = false;
                        for x in &mut *v {
                            if x.name == cur_release.name {
                                found = true;
                                let prev_sem_ver = semver::Version::parse(&x.version).unwrap();
                                if sem_ver == prev_sem_ver {
                                    exist = true;
                                    //replace
                                    //x.version.clone_from(&cur_release.version);
                                }
                                //found break;
                                //break;
                            }
                        }
                        #[allow(clippy::if_same_then_else)]
                        if !found {
                            v.push(model::general_model::Version::new(
                                &cur_release.name,
                                &cur_release.version,
                            ));
                        } else if !exist {
                            v.push(model::general_model::Version::new(
                                &cur_release.name,
                                &cur_release.version,
                            ));
                        }
                    } else {
                        // No vec
                        self.actually_depends_on_map.insert(
                            reverse_dep.clone(),
                            vec![model::general_model::Version::new(
                                &cur_release.name,
                                &cur_release.version,
                            )],
                        );
                    }
                }
            }
        }
    }

    /// insert (dependency, dependent)
    /// notice that: dependent is unique, but dependency should be newest.
    pub async fn insert_reverse_dep(
        &mut self,
        dependency_name: &str,
        dependency_version: &str,
        dependent_name: &str,
        dependent_version: &str,
    ) {
        //let dependency = model::general_model::Version::new(dependency_name, dependency_version);
        let dependent = model::general_model::Version::new(dependent_name, dependent_version);
        self.reverse_depends_on_map
            .entry(dependency_name.to_string())
            .or_default()
            .push((dependency_version.to_string(), dependent));
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct VersionParser {
    version_map: HashMap<String, Vec<String>>,
}

impl VersionParser {
    pub async fn insert_version(&mut self, crate_name: &str, version: &str) {
        self.version_map
            .entry(crate_name.to_string())
            .or_default()
            .push(version.to_string());
    }

    pub(crate) fn exists(&self, name: &str, version: &str) -> bool {
        if let Some(map) = self.version_map.get(name) {
            return map.contains(&version.to_string());
        }
        false
    }

    pub(crate) async fn _remove(&mut self, name: &str) {
        self.version_map.remove(name);
    }

    pub async fn find_latest_matching_version(
        &self,
        target_lib: &str,
        target_version: &str,
    ) -> Option<String> {
        if let Some(lib_map) = self.version_map.get(target_lib) {
            // if the lib exists
            let req_str = if target_version.contains('.') {
                format!("^{target_version}")
            } else {
                format!("{target_version}.*")
            };

            let requirement = match semver::VersionReq::parse(&req_str) {
                Ok(req) => req,
                Err(_) => return None, // 如果无法解析为有效的版本请求，则返回 None
            };

            let mut matching_versions: Vec<semver::Version> = lib_map
                .iter()
                .filter_map(|ver| semver::Version::parse(ver).ok()) // 将所有版本字符串解析为 Version 对象
                .filter(|ver| requirement.matches(ver))
                .collect();

            // Sort the matched versions and return the last (largest) one
            matching_versions.sort();
            return matching_versions.last().map(|v| v.to_string());
        }
        None
    }
}

impl VersionUpdater {
    #[allow(unused)]
    pub fn calculate_memory_usage(&self) -> String {
        let stack_size = mem::size_of_val(self);

        let mut heap_size = 0;

        // Calculate heap size for reverse_depends_on_map
        for (key, value) in &self.reverse_depends_on_map {
            heap_size += key.capacity() * mem::size_of::<char>(); // String capacity
            heap_size +=
                value.capacity() * mem::size_of::<(String, model::general_model::Version)>();
            for (s, _) in value {
                heap_size += s.capacity() * mem::size_of::<char>(); // String capacity
            }
        }

        // Calculate heap size for actually_depends_on_map
        for value in self.actually_depends_on_map.values() {
            heap_size += mem::size_of::<model::general_model::Version>(); // Key size
            heap_size += value.capacity() * mem::size_of::<model::general_model::Version>();
        }

        format!(" [Version Updater: {}] ", stack_size + heap_size)
            + &self.version_parser.calculate_memory_usage()
    }
}
impl VersionParser {
    #[allow(unused)]
    fn calculate_memory_usage(&self) -> String {
        let stack_size = mem::size_of_val(self);

        let mut heap_size = 0;

        // Calculate heap size for version_map
        for (key, value) in &self.version_map {
            heap_size += key.capacity() * mem::size_of::<char>(); // String capacity
            heap_size += value.capacity() * mem::size_of::<String>();
            for s in value {
                heap_size += s.capacity() * mem::size_of::<char>(); // String capacity
            }
        }

        format!(" [Version Parser: {}] ", stack_size + heap_size)
    }
}

#[cfg(test)]
mod tests {
    use super::VersionParser;

    #[tokio::test]
    async fn test_insert_and_find_version() {
        let mut parser = VersionParser::default();
        parser.insert_version("crate_a", "1.0.1").await;
        parser.insert_version("crate_a", "1.1.1").await;
        parser.insert_version("crate_a", "1.2.1").await;
        parser.insert_version("crate_a", "1.2.2").await;

        // Test finding the latest exact version
        assert_eq!(
            parser.find_latest_matching_version("crate_a", "1.2").await,
            Some("1.2.2".to_string())
        );
        assert_eq!(
            parser.find_latest_matching_version("crate_a", "1").await,
            Some("1.2.2".to_string())
        );

        // Test finding versions when there's no match
        assert_eq!(
            parser.find_latest_matching_version("crate_a", "2.0").await,
            None
        );

        // Test finding versions with a precise match
        parser.insert_version("crate_b", "2.0.0").await;
        parser.insert_version("crate_b", "2.0.1").await;
        assert_eq!(
            parser
                .find_latest_matching_version("crate_b", "2.0.1")
                .await,
            Some("2.0.1".to_string())
        );

        assert_eq!(
            parser.find_latest_matching_version("crate_b", "2").await,
            Some("2.0.1".to_string())
        );
        assert_eq!(
            parser.find_latest_matching_version("crate_c", "2").await,
            None
        );
    }
}
