use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Represents detailed information about a Rust crate.
///
/// This structure includes metadata fields that describe a crate, such as its name,
/// current version, description, and various URLs related to its documentation,
/// repository, and license, along with a count of its dependencies.
#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct Program {
    /// The id
    pub id: String,
    /// The name of the crate.
    pub name: String,
    /// An optional description of the crate.
    pub description: Option<String>,
    /// The namespace of the crate, such tokio-rs/tokio
    pub namespace: Option<String>,
    /// The current version of the crate.
    pub max_version: Option<String>,
    /// An optional URL pointing to the crate's source code repository.
    pub github_url: Option<String>,
    /// mega URL
    pub mega_url: Option<String>,
    /// An optional URL pointing to the crate's documentation.
    pub doc_url: Option<String>,
}

impl Program {
    /// Constructs a new `CrateInfo`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        name: String,
        description: Option<String>,
        namespace: Option<String>,
        max_version: Option<String>,
        github_url: Option<String>,
        mega_url: Option<String>,
        doc_url: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            namespace,
            max_version,
            github_url,
            mega_url,
            doc_url,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub enum UProgram {
    Library(Library),
    Application(Application),
}
impl CrateType2Idx for UProgram {
    fn index(&self) -> usize {
        match self {
            Self::Library(_) => 0,
            Self::Application(_) => 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct Library {
    /// The id
    pub id: String,
    /// The name of the library.
    pub name: String,
    /// The number of downloads.
    pub downloads: i64,
    /// An optional URL pointing to the library's crates.io page.
    pub cratesio: Option<String>,
}

impl Library {
    pub fn new(id: &str, name: &str, downloads: i64, cratesio: Option<&str>) -> Self {
        Library {
            id: id.to_string(),
            name: name.to_string(),
            downloads,
            cratesio: cratesio.map(|s| s.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct Application {
    /// The id
    pub id: String,
    /// The name of the application.
    pub name: String,
}

impl Application {
    /// Creates a new `Application` instance.
    pub fn new(id: String, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub enum UVersion {
    LibraryVersion(LibraryVersion),
    ApplicationVersion(ApplicationVersion),
}

impl CrateType2Idx for UVersion {
    fn index(&self) -> usize {
        match self {
            Self::LibraryVersion(_) => 0,
            Self::ApplicationVersion(_) => 1,
        }
    }
}

impl UVersion {
    pub fn get_name_and_version(&self) -> String {
        match self {
            UVersion::LibraryVersion(l) => l.name_and_version.clone(),
            UVersion::ApplicationVersion(a) => a.name_and_version.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct LibraryVersion {
    /// The id
    pub id: String,
    /// The name and version of the crate.
    pub name_and_version: String,
    /// The name of the library.
    pub name: String,
    /// The version of the library.
    pub version: String,
    /// The documentation URL for the library.
    pub documentation: String,
}

impl LibraryVersion {
    /// Create a new `LibraryVersion` instance.
    pub fn new(id: String, name: &str, version: &str, documentation: &str) -> Self {
        LibraryVersion {
            id,
            name_and_version: name.to_string() + "/" + version,
            name: name.to_string(),
            version: version.to_string(),
            documentation: documentation.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct ApplicationVersion {
    /// The id
    pub id: String,
    /// The name and version of the crate.
    pub name_and_version: String,
    /// The name
    pub name: String,
    /// The version
    pub version: String,
}

impl ApplicationVersion {
    /// Create a new `ApplicationVersion` instance.
    pub fn new(id: String, name: String, version: String) -> Self {
        ApplicationVersion {
            id,
            name_and_version: name.to_string() + "/" + &version,
            name,
            version,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, ToSchema)]
pub struct Version {
    /// The name and version of the crate.
    pub name_and_version: String,
}

impl Version {
    /// Creates a new `Version` instance.
    pub fn new(name_and_version: &str) -> Self {
        Version {
            name_and_version: name_and_version.to_string(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct HasType {
    pub SRC_ID: String,
    pub DST_ID: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct HasVersion {
    pub SRC_ID: String,
    pub DST_ID: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct HasDepVersion {
    pub SRC_ID: String,
    pub DST_ID: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DependsOn {
    pub SRC_ID: String,
    pub DST_ID: String,
}

pub trait CrateType2Idx {
    fn index(&self) -> usize;
    fn is_library(&self) -> bool {
        self.index() == 0
    }
}
