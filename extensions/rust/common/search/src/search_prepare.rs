use std::env;
use tokio_postgres::Client as PgClient;

use crate::embedding;

pub struct SearchPrepare<'a> {
    pg_client: &'a PgClient,
    table_name: String,
}
impl<'a> SearchPrepare<'a> {
    pub async fn new(pgclient: &'a PgClient) -> Self {
        let table_name = env::var("TABLE_NAME").unwrap_or_else(|_| "crates".to_string());
        SearchPrepare {
            pg_client: pgclient,
            table_name,
        }
    }
    //检查crates表是否存在

    pub async fn prepare_tsv(&self) -> Result<(), Box<dyn std::error::Error>> {
        let table_exists = self.crates_table_exists().await?;
        if !table_exists {
            return Err("crates table not exists".into());
        }
        self.add_tsv_column().await?;
        self.set_tsv_column().await?;
        self.create_tsv_index().await?;
        Ok(())
    }

    pub async fn prepare_embedding(&self) -> Result<(), Box<dyn std::error::Error>> {
        let table_exists = self.crates_table_exists().await?;
        if !table_exists {
            return Err("crates table not exists".into());
        }
        self.add_pgvector_extension().await?;
        self.add_embedding_column().await?;
        self.set_embedding_column().await?;
        self.create_embedding_index().await?;
        Ok(())
    }

    pub async fn crates_table_exists(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let query = format!(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_name = '{}'
            )",
            self.table_name
        );
        let rows = self.pg_client.query(&query, &[]).await?;
        Ok(rows[0].get(0))
    }

    // 功能二：添加tsv列
    pub async fn add_tsv_column(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS tsv tsvector",
            self.table_name
        );
        self.pg_client.execute(&query, &[]).await?;
        Ok(())
    }

    // 功能三：为tsv列创建索引
    pub async fn create_tsv_index(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_tsv ON {} USING gin(tsv)",
            self.table_name, self.table_name
        );
        self.pg_client.execute(&query, &[]).await?;
        Ok(())
    }

    // 功能四：设置tsv为crates中name属性和description属性的全文搜索tsvector
    pub async fn set_tsv_column(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = format!(
            "UPDATE {} SET tsv = 
                setweight(to_tsvector('english', coalesce(name, '')), 'A') || 
                setweight(to_tsvector('english', coalesce(description, '')), 'B')",
            self.table_name
        );
        self.pg_client.execute(&query, &[]).await?;
        Ok(())
    }

    // 功能五：添加embedding列
    pub async fn add_embedding_column(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS embedding vector(1536)",
            self.table_name
        );
        self.pg_client.execute(&query, &[]).await?;
        Ok(())
    }

    pub async fn set_embedding_column(&self) -> Result<(), Box<dyn std::error::Error>> {
        embedding::update_all_crate_embeddings(self.pg_client).await
    }
    // 功能六：为embedding列创建索引
    pub async fn create_embedding_index(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_embedding ON {} USING ivfflat (embedding)",
            self.table_name, self.table_name
        );
        self.pg_client.execute(&query, &[]).await?;
        Ok(())
    }
    // 功能七:检查是否有crates表,crates表是否有tsv列,是否有embedding列,是否有索引,是否有数据,如果都有,返回true,否则返回false
    pub async fn check_ok(&self) -> bool {
        let table_exists = self.crates_table_exists().await.unwrap_or(false);
        if !table_exists {
            return false;
        }

        let tsv_column_exists = self
            .pg_client
            .query(
                &format!(
                    "SELECT EXISTS (
                        SELECT FROM information_schema.columns 
                        WHERE table_name = '{}' AND column_name = 'tsv'
                    )",
                    self.table_name
                ),
                &[],
            )
            .await
            .map(|rows| rows[0].get(0))
            .unwrap_or(false);
        if !tsv_column_exists {
            return false;
        }

        let embedding_column_exists = self
            .pg_client
            .query(
                &format!(
                    "SELECT EXISTS (
                        SELECT FROM information_schema.columns 
                        WHERE table_name = '{}' AND column_name = 'embedding'
                    )",
                    self.table_name
                ),
                &[],
            )
            .await
            .map(|rows| rows[0].get(0))
            .unwrap_or(false);
        if !embedding_column_exists {
            return false;
        }

        let tsv_index_exists = self
            .pg_client
            .query(
                &format!(
                    "SELECT EXISTS (
                        SELECT FROM pg_indexes 
                        WHERE tablename = '{}' AND indexname = 'idx_{}_tsv'
                    )",
                    self.table_name, self.table_name
                ),
                &[],
            )
            .await
            .map(|rows| rows[0].get(0))
            .unwrap_or(false);
        if !tsv_index_exists {
            return false;
        }

        let embedding_index_exists = self
            .pg_client
            .query(
                &format!(
                    "SELECT EXISTS (
                        SELECT FROM pg_indexes 
                        WHERE tablename = '{}' AND indexname = 'idx_{}_embedding'
                    )",
                    self.table_name, self.table_name
                ),
                &[],
            )
            .await
            .map(|rows| rows[0].get(0))
            .unwrap_or(false);
        if !embedding_index_exists {
            return false;
        }

        true
    }

    // 功能八：为crates表添加pgvector扩展
    pub async fn add_pgvector_extension(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = "CREATE EXTENSION IF NOT EXISTS vector";
        self.pg_client.execute(query, &[]).await?;
        Ok(())
    }
}
