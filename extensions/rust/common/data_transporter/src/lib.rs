mod data_packer;
mod data_reader;
pub mod db;
mod handler;
mod transporter;
mod redis_store;

use model::tugraph_model::UVersion;
use search::search_prepare;
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
pub use transporter::Transporter;

use crate::data_reader::DataReader; // 确保导入你的 DataReader
use crate::db::db_connection_config_from_env;
use crate::handler::ApiHandler;

use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Deserialize, Debug, ToSchema)]
pub struct Query {
    query: String,
    pagination: Pagination,
}
#[derive(Deserialize, Debug, ToSchema)]
pub struct Pagination {
    page: usize,
    per_page: usize,
}
#[derive(Deserialize, Debug, ToSchema,Serialize,Clone)]
pub struct Loginfo {
    email: String,
    image:String,
    name: String,
}
#[derive(Deserialize, Debug, ToSchema,Serialize,Clone)]
pub struct Userinfo {
    user:Loginfo,
    expires:String,
}
#[derive(Deserialize, Debug, ToSchema,Serialize,Clone)]
pub struct UploadedCrate {
    name:String,
    time:String,
}
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Root {
    requestBody: RequestBody,
}

#[derive(Debug, Deserialize)]
struct RequestBody {
    session: Userinfo,
}
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct RequestBody2 {
    requestBody: String,
}
async fn get_tugraph_api_handler() -> ApiHandler {
    let tugraph_bolt_url = &std::env::var("TUGRAPH_BOLT_URL").unwrap();
    let tugraph_user_name = &std::env::var("TUGRAPH_USER_NAME").unwrap();
    let tugraph_user_password = &std::env::var("TUGRAPH_USER_PASSWORD").unwrap();
    let tugraph_cratespro_db = &std::env::var("TUGRAPH_CRATESPRO_DB").unwrap();
    let reader = DataReader::new(
        tugraph_bolt_url,
        tugraph_user_name,
        tugraph_user_password,
        tugraph_cratespro_db,
    )
    .await
    .unwrap();
    ApiHandler::new(reader).await
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handler::get_cves,
        handler::get_all_crates,
        //handler::get_graph,
        handler::get_crate_details,
        handler::query_crates,
        //handler::get_graph,
        //route::get_version_page,
        // route::get_graph,
        // route::get_direct_dep_for_graph,
        // route::new_get_crates_front_info,
        // route::new_get_dependency,
        // route::dependency_cache,
        // route::new_get_dependent,
        // route::dependent_cache,
        // route::query_crates,

        // route::get_crate_details,
    ),
    components(
        schemas(
            model::tugraph_model::Program,
            db::Allcve,
            handler::Versionpage,
            //handler::Deptree,
            //handler::Crateinfo,
            handler::DependencyInfo,
            handler::DependentInfo,
            handler::Crateinfo,
            model::tugraph_model::UProgram,
            VersionInfo,
            Query,
            handler::QueryCratesInfo,
            //handler::Deptree,
            // Query, 
            // Pagination,
            // route::QueryCratesInfo,
            // route::QueryData,
            // route::QueryItem,
            // route::DependencyInfo,
            // route::DependencyCrateInfo,
            // route::DependentInfo,
            // route::DependentData,
            // route::Crateinfo,
            
            // route::Versionpage,
            // route::NewRustsec,
            // NameVersion,
            // route::Deptree,
        )
    ),
    tags(
        (name = "crates", description = "Crates API"),
        (name = "dependencies", description = "Dependencies API"),
        (name = "search", description = "Search API"),
        (name = "security", description = "Security API"),
        (name = "versions", description = "Version API"),
        //(name = "upload", description = "Upload API"),
    )
)]
struct ApiDoc;

pub async fn run_api_server() -> std::io::Result<()> {
    tracing::info!("Start run_api_server");
    let db_connection_config = db_connection_config_from_env();
    let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
        .await
        .unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });
    let pre_search = search_prepare::SearchPrepare::new(&client).await;
    pre_search.prepare_tsv().await.unwrap();
    HttpServer::new(move || {
        tracing::info!("start route");
        App::new()
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
            .route(
                "/api/cvelist",
                web::get().to(
                    || async move {
                        handler::get_cves().await
                    },
                ),
            )
            .route(
                "/api/crates",
                web::get().to(|| async move {
                    handler::get_all_crates().await
                }),
            )
            .route(
                "/api/crates/{cratename}",
                web::get().to(
                    |name: web::Path<String>| async move {
                        handler::get_crate_details(name.into_inner().into()).await
                    },
                ),
            )
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/versions", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,cratename, version) = path.into_inner();
                handler::new_get_version_page(nsfront,nsbehind,cratename,version).await
            }))
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/dependencies/graphpage", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,cratename, version) = path.into_inner();
                handler::new_get_graph(nsfront,nsbehind,cratename,version).await
            }))

            .route(
                "/api/submit",
                web::post().to(
                    | payload: Multipart| async move {
                        handler::upload_crate(payload).await
                    },
                ),
            )
            .route(
                "/api/submitCrate",
                web::post().to(
                    | payload: Multipart| async move {
                        tracing::info!("enter submitcrate");
                        handler::upload_crate(payload).await
                    },
                ),
            )
            .route("/api/submitUserinfo", web::post().to(
                |payload: String| async move{
                    //web::Json<Userinfo>
                    tracing::info!("enter submitUserinfo");
                    tracing::info!("payload:{}",payload.clone());
                    let query:Root = serde_json::from_str(&payload).unwrap();
                    tracing::info!("userinfo {:?}",query);
                    handler::submituserinfo(query.requestBody.session).await
            },),)
            .route("/api/profile", web::post().to(
                |payload: String| async move{
                    tracing::info!("enter profile");
                    tracing::info!("payload:{}",payload.clone());
                    let query:RequestBody2 = serde_json::from_str(&payload).unwrap();
                    tracing::info!("profile email:{}",query.requestBody.clone());
                    handler::query_upload_crate(query.requestBody).await
            },),)
            .route("/api/search", web::post().to(
                |payload: web::Json<Query>| async move{
                    let query = payload.into_inner();
                    handler::query_crates(query).await
            },),)
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/dependencies", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,cratename, version) = path.into_inner();
                handler::dependency_redis_cache(cratename,version,nsfront,nsbehind).await
            }))
            
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/dependencies/graph", 
            web::get().to(|_path: web::Path<(String, String,String,String)>|async move{
                HttpResponse::Ok().json(())
            }))
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/dependents", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,cratename, version) = path.into_inner();
                handler::dependent_redis_cache(cratename,version,nsfront,nsbehind).await
            }))
            
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,cratename, version) = path.into_inner();
                handler::new_get_crates_front_info_from_redis(cratename,version,nsfront,nsbehind).await
            }))
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/senseleak", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,_cratename,_versionn) = path.into_inner();
                handler::get_senseleak(nsfront, nsbehind).await
            }))
            .route("/api/graph/{cratename}/{version}/direct", 
            web::get().to(|path: web::Path<(String,String)>|async move{
                let (cratename, version) = path.into_inner();
                handler::get_direct_dep_for_graph(cratename,version).await
            }))
            .route("/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/unsafechecker", 
            web::get().to(|path: web::Path<(String, String,String,String)>|async move{
                let (nsfront,nsbehind,cratename,version) = path.into_inner();
                handler::get_mirchecker(nsfront, nsbehind,cratename,version).await
            }))
    })
    .bind("0.0.0.0:6888")?
    .run()
    .await
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema,Hash,PartialEq,Eq)]
pub struct NameVersion {
    pub name: String,
    pub version: String,
}

impl NameVersion {
    // 解析 "name/version" 格式的字符串
    pub fn from_string(name_version: &str) -> Option<Self> {
        let parts: Vec<&str> = name_version.split('/').collect();
        if parts.len() == 2 {
            Some(NameVersion {
                name: parts[0].to_string(),
                version: parts[1].to_string(),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    pub version_base: UVersion,
    pub dependencies: Vec<NameVersion>,
}
