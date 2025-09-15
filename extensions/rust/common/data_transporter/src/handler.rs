use std::collections::HashSet;
#[allow(unused_imports)]
use std::env;
use std::error::Error;
//use std::error::Error;
use std::time::Instant;

use crate::data_reader::{DataReader, DataReaderTrait};
use crate::db::{db_connection_config_from_env, DBHandler};
use crate::redis_store::{get_redis_connection, RedisHandler};
use crate::{get_tugraph_api_handler, NameVersion, Userinfo};
use crate::{Query, VersionInfo};
use actix_multipart::{Field, Multipart};
use actix_web::http::header::ContentDisposition;
use actix_web::{web, HttpResponse, Responder};
use futures_util::StreamExt;
//use model::repo_sync_model;
//use model::repo_sync_model::CrateType;
use model::tugraph_model::{Program, UProgram};
//use repo_import::ImportDriver;
use sanitize_filename::sanitize;
use search::crates_search::RecommendCrate;
//use search::crates_search::RecommendCrate;
use search::crates_search::SearchModule;
use search::crates_search::SearchSortCriteria;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::io::Cursor;
use std::io::Read;
//use std::time::Instant;
//use semver::Version;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio_postgres::NoTls;
use utoipa::ToSchema;
use zip::ZipArchive;
pub struct ApiHandler {
    reader: DataReader,
}
impl ApiHandler {
    pub async fn new(reader: DataReader) -> Self {
        Self { reader }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct QueryCratesInfo {
    code: u32,
    message: String,
    data: QueryData,
}
#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct QueryData {
    total_page: usize,
    items: Vec<QueryItem>,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct QueryItem {
    name: String,
    version: String,
    date: String,
    nsfront: String,
    nsbehind: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DependencyCrateInfo {
    pub crate_name: String,
    pub version: String,
    pub relation: String,
    pub license: String,
    pub dependencies: usize,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DependencyInfo {
    pub direct_count: usize,
    pub indirect_count: usize,
    pub data: Vec<DependencyCrateInfo>,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DependentInfo {
    pub direct_count: usize,
    pub indirect_count: usize,
    pub data: Vec<DependentData>,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DependentData {
    pub crate_name: String,
    pub version: String,
    pub relation: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Crateinfo {
    pub crate_name: String,
    pub description: String,
    pub dependencies: DependencyCount,
    pub dependents: DependentCount,
    pub cves: Vec<NewRustsec>,
    pub dep_cves: Vec<NewRustsec>,
    pub license: String,
    pub github_url: String,
    pub doc_url: String,
    pub versions: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DependencyCount {
    pub direct: usize,
    pub indirect: usize,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DependentCount {
    pub direct: usize,
    pub indirect: usize,
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash, ToSchema)]
pub struct RustSec {
    pub id: String,
    pub cratename: String,
    pub patched: String,
    pub aliases: Vec<String>,
    pub small_desc: String,
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash, ToSchema)]
pub struct NewRustsec {
    pub id: String,
    pub subtitle: String,
    pub reported: String,
    pub issued: String,
    pub package: String,
    pub ttype: String,
    pub keywords: String,
    pub aliases: String,
    pub reference: String,
    pub patched: String,
    pub unaffected: String,
    pub description: String,
    pub url: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Deptree {
    pub name_and_version: String,
    pub cve_count: usize,
    #[schema(value_type = Vec<Deptree>)]
    pub direct_dependency: Vec<Deptree>,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Versionpage {
    pub version: String,
    pub updated_at: String,
    pub downloads: String,
    pub dependents: usize,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SenseleakRes {
    pub exist: bool,
    pub res: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct MircheckerRes {
    pub run_state: bool,
    pub exist: bool,
    pub res: Vec<String>,
}
impl MircheckerRes {
    pub fn new() -> Self {
        Self {
            run_state: true,
            exist: false,
            res: Vec::new(),
        }
    }
}
/// 获取cve信息
#[utoipa::path(
    get,
    path = "/api/cvelist",
    responses(
        (status = 200, description = "成功获取crate信息", body = crate::db::Allcve),
        (status = 404, description = "未找到crate信息")
    ),
    tag = "security"
)]
pub async fn get_cves() -> impl Responder {
    let _handler = get_tugraph_api_handler().await;

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
    let dbhd = DBHandler { client };
    let cves = dbhd.get_all_cvelist().await.unwrap();

    HttpResponse::Ok().json(cves)
}

/// 获取所有crates
#[utoipa::path(
    get,
    path = "/api/crates",
    responses(
        (status = 200, description = "成功获取所有crate的id", body = Vec<model::tugraph_model::Program>),
        (status = 404, description = "未找到crate id")
    ),
    tag = "crates"
)]
pub async fn get_all_crates() -> impl Responder {
    tracing::info!("get all crates func run");
    let handler = get_tugraph_api_handler().await;
    let ids = handler.reader.get_all_programs_id().await;

    let mut programs = vec![];
    for id in &ids {
        let program = handler.reader.get_program(id).await.unwrap();
        programs.push(program);
    }

    tracing::info!("finish get all crates func");

    HttpResponse::Ok().json(programs) // 返回 JSON 格式
}

/// 获取crate详细信息,ok
#[utoipa::path(
    get,
    path = "/api/crates/{cratename}",
    params(
        ("cratename" = String, Path, description = "crate 名称")
    ),
    responses(
        (status = 200, description = "成功获取crate详细信息", body= (Program, UProgram, Vec<VersionInfo>)),
        (status = 404, description = "未找到crate"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "crates"
)]
pub async fn get_crate_details(crate_name: web::Path<String>) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
    match handler.reader.get_program(&crate_name).await {
        Ok(program) => {
            match handler.reader.get_type(&crate_name).await {
                Ok((uprogram, islib)) => {
                    match handler.reader.get_versions(&crate_name, islib).await {
                        Ok(versions) => {
                            HttpResponse::Ok().json((program, uprogram, versions))
                            // 返回 JSON 格式
                        }
                        Err(_) => {
                            HttpResponse::InternalServerError().body("Failed to get versions.")
                        }
                    }
                }
                Err(_) => HttpResponse::InternalServerError().body("Failed to get type."),
            }
        }
        Err(_) => HttpResponse::NotFound().body("Crate not found."),
    }
}

/// 获取直接依赖关系图,ok
#[utoipa::path(
    get,
    path = "/api/graph/{cratename}/{version}/direct",
    params(
        ("cratename" = String, Path, description = "crate 名称"),
        ("version" = String, Path, description = "版本号")
    ),
    responses(
        (status = 200, description = "成功获取依赖关系图", body = Vec<NameVersion>),
        (status = 404, description = "未找到依赖关系图")
    ),
    tag = "dependencies"
)]
pub async fn get_direct_dep_for_graph(nname: String, nversion: String) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
    let name_and_version = nname + "/" + &nversion;
    let res = handler
        .reader
        .get_direct_dependency_nodes(&name_and_version)
        .await
        .unwrap();
    HttpResponse::Ok().json(res)
}



/// 查询 crates
#[utoipa::path(
    post,
    path = "/api/search",
    request_body = Query,
    responses(
        (status = 200, description = "查询成功", body = QueryCratesInfo),
        (status = 400, description = "无效的查询参数")
    ),
    tag = "search"
)]
pub async fn query_crates(q: Query) -> impl Responder {
    let _handler = get_tugraph_api_handler().await;
    //add yj's search module
    let name = q.query;
    let page = q.pagination.page;
    let per_page = q.pagination.per_page;
    tracing::info!("name:{},page:{},per_page:{}", name, page, per_page);
    let db_connection_config = db_connection_config_from_env();
    let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
        .await
        .unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });
    let start_time2 = Instant::now();
    let question = name.clone();
    let search_module = SearchModule::new(&client).await;
    let res = search_module
        .search_crate(&question, SearchSortCriteria::Relavance)
        .await
        .unwrap();
    tracing::trace!("search need time:{:?}", start_time2.elapsed());
    let mut seen = HashSet::new();
    let uniq_res: Vec<RecommendCrate> = res
        .into_iter()
        .filter(|x| seen.insert((x.name.clone(), x.namespace.clone())))
        .collect();
    tracing::trace!("total programs: {}", uniq_res.len());
    let mut gettotal_page = uniq_res.len() / per_page;
    if uniq_res.is_empty() || uniq_res.len() % per_page != 0 {
        gettotal_page += 1;
    }
    let mut getitems = vec![];
    for i in (page - 1) * 20..(page - 1) * 20 + 20 {
        if i >= uniq_res.len() {
            break;
        }
        let mut mv = vec![];
        let program_name = uniq_res[i].clone().name;
        let getnamespace = uniq_res[i].clone().namespace;
        let parts: Vec<&str> = getnamespace.split('/').collect();
        let nsf = parts[0].to_string();
        let nsb = parts[1].to_string();

        mv.push(uniq_res[i].clone().max_version);

        if mv[0].clone() == *"null" {
            mv[0] = "0.0.0".to_string();
        }
        let query_item = QueryItem {
            name: program_name.clone(),
            version: mv[0].clone(),
            date: "".to_string(),
            nsfront: nsf,
            nsbehind: nsb,
        };
        getitems.push(query_item);
    }
    let response = QueryCratesInfo {
        code: 200,
        message: "成功".to_string(),
        data: QueryData {
            total_page: gettotal_page,
            items: getitems,
        },
    };

    HttpResponse::Ok().json(response)
}
//post of upload
#[allow(clippy::let_unit_value)]
pub async fn upload_crate(mut payload: Multipart) -> impl Responder {
    tracing::info!("enter upload crate");
    use futures_util::StreamExt as _;
    let mut upload_time: Option<String> = None;
    let mut user_email: Option<String> = None;
    let mut github_link: Option<String> = None;
    let mut file_name: Option<String> = None;
    while let Some(Ok(mut field)) = payload.next().await {
        tracing::info!("enter while");
        if let Some(content_disposition) = field.content_disposition().cloned() {
            tracing::info!("enter first if");
            if let Some(name) = content_disposition.get_name() {
                tracing::info!("enter second if");
                match name {
                    "file" => {
                        tracing::info!("enter match file");
                        file_name = process_file_of_upload_crate(&content_disposition, &mut field)
                            .await
                            .unwrap();
                        // analyze
                    }
                    "githubLink" => {
                        github_link = process_githublink_of_upload_crate(&mut field)
                            .await
                            .unwrap();
                    }
                    "uploadTime" => {
                        tracing::info!("enter match uploadtime");
                        upload_time = process_uploadtime_of_upload_crate(&mut field)
                            .await
                            .unwrap();
                    }
                    "user_email" => {
                        tracing::info!("enter match user_email");
                        user_email = process_useremail_of_upload_crate(&mut field).await.unwrap();
                    }
                    _ => {
                        tracing::info!("enter match nothing");
                    }
                }
            }
        }
    }
    let _ = process_insertintopg_of_upload_crate(file_name, upload_time, github_link, user_email)
        .await
        .unwrap();
    HttpResponse::Ok().json(())
}
pub async fn process_insertintopg_of_upload_crate(
    file_name: Option<String>,
    upload_time: Option<String>,
    github_link: Option<String>,
    user_email: Option<String>,
) -> Result<(), Box<dyn Error>> {
    if let Some(filename) = file_name {
        tracing::info!("enter 1/2 if let");
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
        if let Some(uploadtime) = upload_time.clone() {
            tracing::info!("enter upload time:{}", uploadtime.clone());
            if let Some(useremail) = user_email.clone() {
                tracing::info!("enter user email:{}", useremail.clone());
                dbhandler
                    .client
                    .execute(
                        "INSERT INTO uploadedcrate(email,filename,uploadtime) VALUES ($1, $2,$3);",
                        &[&useremail.clone(), &filename.clone(), &uploadtime.clone()],
                    )
                    .await
                    .unwrap();
            }
        }
    };
    if let Some(githublink) = github_link {
        tracing::info!("enter 2/2 if let");
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
        if let Some(uploadtime) = upload_time.clone() {
            if let Some(useremail) = user_email.clone() {
                dbhandler
                    .client
                    .execute(
                        "INSERT INTO uploadedurl(email,githuburl,uploadtime) VALUES ($1, $2,$3);",
                        &[&useremail.clone(), &githublink.clone(), &uploadtime.clone()],
                    )
                    .await
                    .unwrap();
            }
        }
    }
    Ok(())
}
#[allow(unused_assignments)]
pub async fn process_useremail_of_upload_crate(
    field: &mut Field,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut user_email: Option<String> = None;
    let mut email_data = Vec::new();

    while let Some(chunk) = field.next().await {
        let data = chunk.unwrap();
        email_data.extend_from_slice(&data);
    }
    user_email = Some(String::from_utf8(email_data).unwrap_or_default());
    tracing::info!("user_email:{:?}", user_email);
    Ok(user_email)
}
#[allow(unused_assignments)]
pub async fn process_uploadtime_of_upload_crate(
    field: &mut Field,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut upload_time: Option<String> = None;
    let mut time_data = Vec::new();

    while let Some(chunk) = field.next().await {
        let data = chunk.unwrap();
        time_data.extend_from_slice(&data);
    }
    upload_time = Some(String::from_utf8(time_data).unwrap_or_default());
    tracing::info!("uploadtime:{:?}", upload_time);
    Ok(upload_time)
}
#[allow(unused_assignments)]
pub async fn process_githublink_of_upload_crate(
    field: &mut Field,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut github_link: Option<String> = None;
    let mut url_data = Vec::new();
    while let Some(chunk) = field.next().await {
        let data = chunk.unwrap();
        url_data.extend_from_slice(&data);
    }
    github_link = Some(String::from_utf8(url_data).unwrap_or_default());
    Ok(github_link)
}
#[allow(unused_assignments)]
pub async fn process_file_of_upload_crate(
    content_disposition: &ContentDisposition,
    field: &mut Field,
) -> Result<Option<String>, Box<dyn Error>> {
    tracing::info!("enter match file");
    let mut file_name: Option<String> = None;
    let filename = if let Some(file_name) = content_disposition.get_filename() {
        file_name.to_string()
    } else {
        "default.zip".to_string()
    };
    tracing::info!("filename:{}", filename.clone());
    let sanitized_filename = sanitize(filename.clone());
    file_name = Some(filename.clone());
    tracing::info!("file_name:{:?}", file_name.clone());
    if sanitized_filename.ends_with(".zip") {
        tracing::info!("enter file zip");
        let zip_filepath = format!("target/zip/upload/{sanitized_filename}");
        let _ = tokio::fs::create_dir_all("target/zip/upload/").await;
        let mut f = tokio::fs::File::create(&zip_filepath).await.unwrap();
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f.write_all(&data).await.unwrap();
        }
        let parts: Vec<&str> = sanitized_filename.split('.').collect();
        let mut filename = "".to_string();
        if parts.len() >= 2 {
            filename = parts[0].to_string();
            tracing::info!("filename without zip: {}", filename);
        }
        let mut zip_file = tokio::fs::File::open(&zip_filepath).await.unwrap();
        let mut buffer = Vec::new();
        zip_file.read_to_end(&mut buffer).await.unwrap();
        let reader = Cursor::new(buffer.clone());
        let mut archive = ZipArchive::new(reader).unwrap();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => {
                    format!("target/www/uploads/{}/{}", filename, path.display())
                }
                None => continue,
            };

            if file.name().ends_with('/') {
                // This is a directory, create it
                tokio::fs::create_dir_all(&outpath).await.unwrap();
            } else {
                // Ensure the parent directory exists
                if let Some(parent) = std::path::Path::new(&outpath).parent() {
                    if !parent.exists() {
                        tokio::fs::create_dir_all(&parent).await.unwrap();
                    }
                }

                // Write the file
                let mut outfile = tokio::fs::File::create(&outpath).await.unwrap();
                while let Ok(bytes_read) = file.read(&mut buffer) {
                    if bytes_read == 0 {
                        break;
                    }
                    outfile.write_all(&buffer[..bytes_read]).await.unwrap();
                }
            }
        }
        tracing::info!("finish match file");
        //send message
    } else {
        tracing::info!("enter else");
        let filepath = format!("/home/rust/output/www/uploads/{sanitized_filename}");
        let mut f = tokio::fs::File::create(&filepath).await.unwrap();

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f.write_all(&data).await.unwrap();
        }
    }
    Ok(file_name)
}
//post of log in
pub async fn submituserinfo(info: Userinfo) -> impl Responder {
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
    tracing::info!("enter submituserinfo and set db client");
    #[allow(clippy::let_unit_value)]
    let _ = dbhandler
        .insert_userinfo_into_pg(info.clone())
        .await
        .unwrap();
    HttpResponse::Ok().json(())
}
pub async fn query_upload_crate(email: String) -> impl Responder {
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
    let mut real_res = vec![];
    let res = dbhandler
        .query_uploaded_crates_from_pg(email.clone())
        .await
        .unwrap();
    for row in res {
        real_res.push(row);
    }
    let res2 = dbhandler
        .query_uploaded_url_from_pg(email.clone())
        .await
        .unwrap();
    for row in res2 {
        real_res.push(row);
    }
    HttpResponse::Ok().json(real_res)
}
pub async fn get_senseleak(nsfront: String, nsbehind: String) -> impl Responder {
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
    let id = nsfront.clone() + "/" + &nsbehind;
    let res = dbhandler.get_senseleak_from_pg(id).await.unwrap();
    let mut exist = true;
    if res.clone() == *"[]" {
        exist = false;
    }
    let return_val = SenseleakRes { exist, res };
    HttpResponse::Ok().json(return_val)
}
#[allow(unused_assignments)]
pub async fn get_mirchecker(
    nsfront: String,
    nsbehind: String,
    name: String,
    version: String,
) -> impl Responder {
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
    let mut return_val = MircheckerRes::new();
    if version.clone() == *"all" {
        let handler = get_tugraph_api_handler().await;
        let namespace = nsfront.clone() + "/" + &nsbehind;
        let mut all_versions = handler
            .reader
            .new_get_lib_version(namespace.clone(), name.clone())
            .await
            .unwrap();
        all_versions.sort_by(|a, b| {
            let version_a = Version::parse(a);
            let version_b = Version::parse(b);
            match (version_a, version_b) {
                (Ok(v_a), Ok(v_b)) => v_b.cmp(&v_a),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => Ordering::Equal,
            }
        });
        let run_state = true;
        let mut exist = false;
        let mut tmp_res = vec![];
        for single_version in all_versions.clone() {
            let id = nsfront.clone() + "/" + &nsbehind + "/" + &name + "/" + &single_version;
            let single_res = dbhandler.get_mirchecker_from_pg(id.clone()).await.unwrap();
            if single_res.contains("warning: [MirChecker]") {
                exist = true;
                tmp_res.push("Version-".to_string() + &single_version + ":\n" + &single_res);
            }
        }
        return_val = MircheckerRes {
            run_state,
            exist,
            res: tmp_res,
        };
    } else {
        let id = nsfront.clone() + "/" + &nsbehind + "/" + &name + "/" + &version;
        let run_state = dbhandler
            .get_mirchecker_run_state_from_pg(id.clone())
            .await
            .unwrap();
        let res = dbhandler.get_mirchecker_from_pg(id.clone()).await.unwrap();
        let mut real_res = vec![];
        let mut exist = false;
        if res.contains("warning: [MirChecker]") {
            exist = true;
            real_res.push(res.clone());
        }
        return_val = MircheckerRes {
            run_state,
            exist,
            res: real_res,
        };
    }
    HttpResponse::Ok().json(return_val)
}

pub async fn new_get_crates_front_info_from_redis(
    nname: String,
    nversion: String,
    nsfront: String,
    nsbehind: String,
) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
    let namespace = nsfront.clone() + "/" + &nsbehind.clone();

    let conn = get_redis_connection().await.unwrap();
    let mut redisconn = RedisHandler { connection: conn };
    let qid = format!("crates_info:{namespace}:{nname}:{nversion}");
    let qres = redisconn.query_from_redis(qid).await.unwrap();
    println!("finish query crates from reids");
    if qres.is_empty() {
        println!("qres is empty");
        let res = handler
            .reader
            .get_crates_front_info_from_tg(
                nname.clone(),
                nversion.clone(),
                nsfront.clone(),
                nsbehind.clone(),
            )
            .await
            .unwrap();
        println!("finish get crates_info from tugraph");
        let val = serde_json::to_string(&res).unwrap();
        redisconn
            .insert_crates_info_into_redis(
                namespace.clone(),
                nname.clone(),
                nversion.clone(),
                val.clone(),
            )
            .await
            .unwrap();
        HttpResponse::Ok().json(res)
    } else {
        let res: Crateinfo = serde_json::from_str(&qres).unwrap();
        HttpResponse::Ok().json(res.clone())
    }
}
pub async fn dependency_redis_cache(
    name: String,
    version: String,
    nsfront: String,
    nsbehind: String,
) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
    let conn = get_redis_connection().await.unwrap();
    let mut redisconn = RedisHandler { connection: conn };
    let namespace = nsfront.clone() + "/" + &nsbehind.clone();
    let qid = format!("dependency:{namespace}:{name}:{version}");
    let res = redisconn.query_from_redis(qid.clone()).await.unwrap();
    if res.is_empty() {
        let res_deps = handler
            .reader
            .get_dependency_from_tg(
                name.clone(),
                version.clone(),
                nsfront.clone(),
                nsbehind.clone(),
            )
            .await
            .unwrap();
        let val = serde_json::to_string(&res_deps).unwrap();
        redisconn
            .insert_dependency_into_redis(
                namespace.clone(),
                name.clone(),
                version.clone(),
                val.clone(),
            )
            .await
            .unwrap();
        HttpResponse::Ok().json(res_deps.clone())
    } else {
        let res_deps: DependencyInfo = serde_json::from_str(&res).unwrap();
        HttpResponse::Ok().json(res_deps.clone())
    }
}
pub async fn new_get_graph(
    nsfront: String,
    nsbehind: String,
    nname: String,
    nversion: String,
) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
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
    let conn = get_redis_connection().await.unwrap();
    let mut redisconn = RedisHandler { connection: conn };
    let namespace = nsfront.clone() + "/" + &nsbehind.clone();
    let qid = format!("dependencygraph:{namespace}:{nname}:{nversion}");
    let qres = redisconn.query_from_redis(qid.clone()).await.unwrap();
    if qres.is_empty() {
        tracing::info!("first time");
        let nav = nname.clone() + "/" + &nversion;
        let rustcve = dbhandler
            .get_direct_rustsec(&nname, &nversion)
            .await
            .unwrap();
        let mut res = Deptree {
            name_and_version: nav.clone(),
            cve_count: rustcve.len(),
            direct_dependency: Vec::new(),
        };
        let mut visited = HashSet::new();
        visited.insert(nav.clone());
        handler
            .reader
            .build_graph(&mut res, &mut visited)
            .await
            .unwrap();
        let graph = serde_json::to_string(&res).unwrap();
        redisconn
            .insert_dependency_graph_into_redis(
                namespace.clone(),
                nname.clone(),
                nversion.clone(),
                graph.clone(),
            )
            .await
            .unwrap();
        HttpResponse::Ok().json(res)
    } else {
        tracing::info!("second time");
        let res_tree: Deptree = serde_json::from_str(&qres).unwrap();
        HttpResponse::Ok().json(res_tree)
    }
}
pub async fn dependent_redis_cache(
    name: String,
    version: String,
    nsfront: String,
    nsbehind: String,
) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
    let conn = get_redis_connection().await.unwrap();
    let mut redisconn = RedisHandler { connection: conn };
    let namespace = nsfront.clone() + "/" + &nsbehind.clone();
    let qid = format!("dependent:{namespace}:{name}:{version}");
    let qres = redisconn.query_from_redis(qid.clone()).await.unwrap();
    if qres.is_empty() {
        let res_deps = handler
            .reader
            .get_dependent_from_tg(
                name.clone(),
                version.clone(),
                nsfront.clone(),
                nsbehind.clone(),
            )
            .await
            .unwrap();
        let val = serde_json::to_string(&res_deps).unwrap();
        redisconn
            .insert_dependent_into_redis(
                namespace.clone(),
                name.clone(),
                version.clone(),
                val.clone(),
            )
            .await
            .unwrap();
        HttpResponse::Ok().json(res_deps)
    } else {
        let res: DependentInfo = serde_json::from_str(&qres).unwrap();
        HttpResponse::Ok().json(res.clone())
    }
}
pub async fn new_get_version_page(
    nsfront: String,
    nsbehind: String,
    nname: String,
    _nversion: String,
) -> impl Responder {
    let handler = get_tugraph_api_handler().await;
    let conn = get_redis_connection().await.unwrap();
    let mut redisconn = RedisHandler { connection: conn };
    let namespace = nsfront.clone() + "/" + &nsbehind.clone();
    let qid = format!("versionpage:{namespace}:{nname}");
    let res = redisconn.query_from_redis(qid.clone()).await.unwrap();
    if res.is_empty() {
        let every_version = handler
            .reader
            .get_version_page_from_tg(nsfront.clone(), nsbehind.clone(), nname.clone())
            .await
            .unwrap();
        let val = serde_json::to_string(&every_version).unwrap();
        redisconn
            .insert_versionpage_into_redis(namespace, nname.clone(), val.clone())
            .await
            .unwrap();
        HttpResponse::Ok().json(every_version)
    } else {
        let every_version: Vec<Versionpage> = serde_json::from_str(&res).unwrap();
        HttpResponse::Ok().json(every_version)
    }
}
