use csv::WriterBuilder;
use lazy_static::lazy_static;
use model::tugraph_model::{Program, UProgram};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Mutex;
use url::Url;

lazy_static! {
    pub static ref NAMESPACE_HASHMAP: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub fn insert_namespace_by_repo_path(key: String, value: String) {
    let mut map = NAMESPACE_HASHMAP.lock().unwrap();
    map.insert(key, value);
}

pub fn get_namespace_by_repo_path(key: &str) -> Option<String> {
    let map = NAMESPACE_HASHMAP.lock().unwrap();
    map.get(key).cloned()
}

lazy_static! {
    pub static ref PROGRAM_HASHMAP: Mutex<HashMap<String, (Program, UProgram)>> =
        Mutex::new(HashMap::new());
}

pub fn insert_program_by_name(key: String, value: (Program, UProgram)) {
    let mut map = PROGRAM_HASHMAP.lock().unwrap();
    map.insert(key, value);
}

pub fn get_program_by_name(key: &str) -> Option<(Program, UProgram)> {
    let map = PROGRAM_HASHMAP.lock().unwrap();
    map.get(key).cloned()
}

pub(crate) fn write_into_csv<T: Serialize + Default + Debug>(
    csv_path: PathBuf,
    programs: Vec<T>,
) -> Result<(), Box<dyn Error>> {
    let serialized = serde_json::to_value(T::default()).unwrap();

    if let serde_json::Value::Object(map) = serialized {
        let field_names: Vec<&str> = map.keys().map(|s| s.as_str()).collect();
        write_to_csv(field_names, csv_path.to_str().unwrap(), false)?;
    }

    for program in &programs {
        let fields = get_fields(program);
        let fields = fields.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        write_to_csv(fields, csv_path.to_str().unwrap(), true)?;
    }

    Ok(())
}

fn write_to_csv(data: Vec<&str>, file_path: &str, append: bool) -> Result<(), Box<dyn Error>> {
    let file = if append {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?
    } else {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path)?
    };

    let mut wtr = WriterBuilder::new()
        .quote_style(csv::QuoteStyle::Necessary)
        .double_quote(true)
        .from_writer(file);

    wtr.write_record(&data)?;
    wtr.flush()?;
    Ok(())
}

fn get_fields<T: Serialize>(item: &T) -> Vec<String> {
    let mut fields = Vec::new();
    let json = json!(item);
    if let serde_json::Value::Object(map) = json {
        fields = map
            .values()
            .map(|value| match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string().trim_matches('"').to_owned(),
            })
            .collect::<Vec<_>>();
    }
    fields
}

/// An auxiliary function
///
/// Extracts namespace e.g. "tokio-rs/tokio" from the git url https://www.github.com/tokio-rs/tokio
pub(crate) fn extract_namespace(url_str: &str) -> Result<String, String> {
    /// auxiliary function
    fn remove_dot_git_suffix(input: &str) -> String {
        let input = if input.ends_with('/') {
            input.strip_suffix('/').unwrap()
        } else {
            input
        };

        let input = if input.ends_with(".git") {
            input.strip_suffix(".git").unwrap().to_string()
        } else {
            input.to_string()
        };
        input
    }

    let url = Url::parse(&remove_dot_git_suffix(url_str))
        .map_err(|e| format!("Failed to parse URL {url_str}: {e}"))?;

    // /tokio-rs/tokio
    let path_segments = url
        .path_segments()
        .ok_or("Cannot extract path segments from URL")?;

    let segments: Vec<&str> = path_segments.collect();
    //println!("{:?}", segments);

    // github URLs is of the format "/user/repo"
    if segments.len() < 2 {
        return Err(format!(
            "URL {url_str} does not include a namespace and a repository name"
        ));
    }

    // join owner name and repo name
    let namespace = format!(
        "{}/{}",
        segments[segments.len() - 2],
        segments[segments.len() - 1]
    );

    Ok(namespace)
}

pub(crate) fn name_join_version(crate_name: &str, version: &str) -> String {
    crate_name.to_string() + "/" + version
}
