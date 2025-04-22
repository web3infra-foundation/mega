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
//use walkdir::WalkDir;

use crate::utils::{CodeItem, ItemType};
use crate::VECT_CLIENT_NODE;

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

    // async fn walk_space(&self, dir: &Path, items: &mut Vec<CodeItem>) -> Result<()> {
    //     for crate_entry in WalkDir::new(dir)
    //         .min_depth(1)
    //         .max_depth(1)
    //         .into_iter()
    //         .filter_map(|e| e.ok())
    //     {
    //         if crate_entry.path().is_dir() {
    //             let crate_path = crate_entry.path();
    //             let crate_name = crate_path.file_name().unwrap().to_str().unwrap();
    //             let repo_path = &crate_path.join(crate_name);
    //             if !repo_path.exists() {
    //                 println!("Skipping crate (repo_path does not exist): {:?}", repo_path);
    //                 continue; // 跳过当前 crate
    //             } else {
    //                 self.walk_dir(repo_path, items).await;
    //             }
    //         }
    //     }
    //     Ok(())
    // }
    async fn walk_space(
        &self,
        crate_entry: &Path,
        items: &mut Vec<CodeItem>,
        crate_version: &str,
    ) -> Result<()> {
        if crate_entry.is_dir() {
            println!("re: {:?}", crate_entry);

            let crate_name = crate_entry.file_name().unwrap().to_str().unwrap();
            let crate_path = crate_entry.join(crate_version);

            if !crate_path.exists() {
                println!(
                    "Skipping crate (crate_path does not exist): {:?}",
                    crate_path
                );
                return Ok(());
            }

            self.walk_dir(&crate_path, items).await?;
        }
        Ok(())
    }

    async fn walk_dir(&self, dir: &Path, items: &mut Vec<CodeItem>) -> Result<()> {
        println!("Walking directory: {:?}", dir);
        log::info!("Current directory: {:?}", std::env::current_dir().unwrap());
        let mut entries = fs::read_dir(dir).await?;
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
    pub crate_version: String,
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
            .walk_space(&self.indexer.crate_path, &mut items, &self.crate_version)
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
