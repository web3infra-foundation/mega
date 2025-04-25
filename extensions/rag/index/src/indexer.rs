use anyhow::{Context, Result};
use async_trait::async_trait;
use dagrs::utils::env::EnvVar;
use dagrs::{Action, Content, NodeId, Output};
use dagrs::{InChannels, OutChannels};
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use syn::{parse_file, Item};
use tokio::fs;

use crate::utils::{CodeItem, ItemType};
use crate::VECT_CLIENT_NODE;

use flate2::read::GzDecoder;
use std::fs::File;
use std::io;
use tar::Archive;

fn unpack_crate_file_to_current_dir(crate_file_path: &Path) -> io::Result<()> {
    let target_dir = crate_file_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Cannot get parent directory of the crate file",
        )
    })?;

    println!("Unpacking to: {:?}", target_dir);

    let tar_gz = File::open(crate_file_path)?;
    let decompressed = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(decompressed);

    archive.unpack(target_dir)?;
    Ok(())
}

#[derive(Clone)]
pub struct CodeIndexer {
    pub crate_path: PathBuf,
}

impl CodeIndexer {
    pub fn new<P: AsRef<Path>>(crate_path: P) -> Self {
        Self {
            crate_path: crate_path.as_ref().to_path_buf(),
        }
    }

    pub async fn index(&self) -> Result<Vec<CodeItem>> {
        let mut items = Vec::new();
        Box::pin(self.walk_dir(&self.crate_path, &mut items)).await?;
        Ok(items)
    }

    async fn walk_space(&self, crate_entry: &Path, items: &mut Vec<CodeItem>) -> Result<()> {
        // Print the path of the crate file being processed
        println!("Processing crate file: {:?}", crate_entry);

        // Verify the crate file exists before proceeding
        assert!(crate_entry.exists());

        // Step 1: Unpack the .crate file to current directory
        unpack_crate_file_to_current_dir(crate_entry)?;

        // Step 2: Extract directory name from crate filename (without .crate extension)
        let dir_name = crate_entry
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid crate file name"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Non-UTF8 crate file name"))?;

        // Step 3: Get parent directory of the crate file
        let parent_dir = crate_entry
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot get parent directory"))?;

        // Step 4: Construct full path to the unpacked directory
        let unpacked_dir = parent_dir.join(dir_name);

        // Print the path of the unpacked directory we'll process
        println!("Processing unpacked directory: {:?}", unpacked_dir);

        // Step 5: Recursively walk through the unpacked directory to process files
        self.walk_dir(&unpacked_dir, items).await?;

        // Step 6: Clean up by removing the unpacked directory after processing
        fs::remove_dir_all(&unpacked_dir).await?;
        Ok(())
    }

    async fn walk_dir(&self, crate_path: &Path, items: &mut Vec<CodeItem>) -> Result<()> {
        let mut entries = fs::read_dir(&crate_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            println!("Found path: {:?}", path);

            if path.is_dir() {
                // Skip target and .git directories
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "target" || n == ".git")
                    .unwrap_or(false)
                {
                    println!("Skipping directory: {:?}", path);
                    continue;
                }
                Box::pin(self.walk_dir(&path, items)).await?;
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "rs")
                .unwrap_or(false)
            {
                println!("Processing Rust file: {:?}", path);
                self.process_rust_file(&path, items).await?;
            }
        }
        Ok(())
    }

    async fn process_rust_file(&self, file_path: &Path, items: &mut Vec<CodeItem>) -> Result<()> {
        println!("Processing file: {:?}", file_path);
        let content = fs::read_to_string(file_path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        let ast = parse_file(&content)
            .with_context(|| format!("Failed to parse Rust file: {:?}", file_path))?;

        println!("Found {} items in file", ast.items.len());
        for item in ast.items {
            match item {
                Item::Fn(fn_item) => {
                    println!("Found function: {}", fn_item.sig.ident);
                    let mut tokens = TokenStream::new();
                    fn_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        fn_item.sig.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Function,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Struct(struct_item) => {
                    println!("Found struct: {}", struct_item.ident);
                    let mut tokens = TokenStream::new();
                    struct_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        struct_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Struct,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Enum(enum_item) => {
                    println!("Found enum: {}", enum_item.ident);
                    let mut tokens = TokenStream::new();
                    enum_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        enum_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Enum,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Trait(trait_item) => {
                    println!("Found trait: {}", trait_item.ident);
                    let mut tokens = TokenStream::new();
                    trait_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        trait_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Trait,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Impl(impl_item) => {
                    println!("Found impl block");
                    let mut tokens = TokenStream::new();
                    impl_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        "impl".to_string(),
                        tokens.to_string(),
                        ItemType::Impl,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Type(type_item) => {
                    println!("Found type alias: {}", type_item.ident);
                    let mut tokens = TokenStream::new();
                    type_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        type_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Type,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Const(const_item) => {
                    println!("Found const: {}", const_item.ident);
                    let mut tokens = TokenStream::new();
                    const_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        const_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Const,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Static(static_item) => {
                    println!("Found static: {}", static_item.ident);
                    let mut tokens = TokenStream::new();
                    static_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        static_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Static,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                Item::Mod(mod_item) => {
                    println!("Found module: {}", mod_item.ident);
                    let mut tokens = TokenStream::new();
                    mod_item.to_tokens(&mut tokens);
                    items.push(self.create_code_item(
                        mod_item.ident.to_string(),
                        tokens.to_string(),
                        ItemType::Module,
                        file_path.to_path_buf(),
                        file_path.metadata()?.len() as usize,
                    ));
                }
                _ => continue,
            }
        }

        Ok(())
    }

    fn create_code_item(
        &self,
        name: String,
        content: String,
        item_type: ItemType,
        file_path: PathBuf,
        line_number: usize,
    ) -> CodeItem {
        CodeItem {
            name,
            content,
            item_type,
            file_path,
            line_number,
            vector: vec![],
        }
    }
}

pub struct WalkDirAction {
    pub indexer: CodeIndexer,
}

#[async_trait]
impl Action for WalkDirAction {
    async fn run(
        &self,
        _in_channels: &mut InChannels,
        out_channels: &mut OutChannels,
        _env: Arc<EnvVar>,
    ) -> Output {
        let mut items = Vec::new();
        match self
            .indexer
            .walk_space(&self.indexer.crate_path, &mut items)
            .await
        {
            Ok(_) => {
                out_channels.broadcast(Content::new(items.clone())).await;
                Output::new(items)
            }
            Err(e) => Output::error(e.to_string()),
        }
    }
}

pub struct ProcessItemsAction;

#[async_trait::async_trait]
impl Action for ProcessItemsAction {
    async fn run(
        &self,
        in_channels: &mut InChannels,
        out_channels: &mut OutChannels,
        _env: Arc<EnvVar>,
    ) -> Output {
        let vect_client_id: &NodeId = _env.get_ref(VECT_CLIENT_NODE).unwrap();

        let items: Vec<CodeItem> = match in_channels
            .map(|content| content.unwrap().into_inner::<Vec<CodeItem>>().unwrap())
            .await
            .first()
            .cloned()
        {
            Some(items) => items.to_vec(),
            None => return Output::error("No items received".to_string()),
        };

        println!("Processing {} items", items.len());
        for item in &items {
            println!(
                "Found {:?} at {:?}:{}",
                item.item_type, item.file_path, item.line_number
            );
            println!("Name: {}", item.name);
            println!("Content:\n{}\n", item.content);
        }

        out_channels.broadcast(Content::new(items.clone())).await;
        out_channels.close(vect_client_id);
        Output::new(items)
    }
}
