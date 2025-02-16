use std::path::PathBuf;

fn main() {
    // Step.1 Build the gschema in a specific path
    // https://gtk-rs.org/gtk4-rs/stable/latest/book/settings.html
    let schema_dir = {
        #[cfg(target_os = "windows")]
        {
            "C:/ProgramData/glib-2.0/schemas/".to_string()
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut dir = std::env::var("HOME").expect("Failed to get HOME");
            dir.push_str("/.local/share/glib-2.0/schemas");
            dir.to_string()
        }
    }
    .parse::<PathBuf>()
    .expect("Failed to get schema directory");

    // Override the old schema
    std::fs::create_dir_all(&schema_dir).expect("Failed to create schema directory");
    std::fs::remove_file(schema_dir.join("org.Web3Infrastructure.Monobean.gschema.xml")).ok();
    std::fs::copy(
        "resources/org.Web3Infrastructure.Monobean.gschema.xml",
        schema_dir.join("org.Web3Infrastructure.Monobean.gschema.xml"),
    )
    .unwrap();

    let output = std::process::Command::new("glib-compile-schemas")
        .arg(schema_dir.to_str().unwrap())
        .output()
        .expect("Failed to compile schemas, did you install the package `glibc2`?");
    if !output.status.success() {
        panic!(
            "Failed to compile schemas: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Step.2 Compile the resources
    let current_dir = std::env!("CARGO_MANIFEST_DIR")
        .parse::<PathBuf>()
        .expect("Failed to get working directory");

    glib_build_tools::compile_resources(
        &[
            current_dir.join("resources"),
            current_dir.join("resources/gtk"),
        ],
        current_dir
            .join("resources/org.Web3Infrastructure.Monobean.gresource.xml")
            .to_str()
            .unwrap(),
        current_dir.join("Monobean.gresource").to_str().unwrap(),
    );

    println!("cargo:info=Resources compiled");
}
