#![allow(dead_code)]

use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use anyhow::Result;

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }
    
    pub async fn migrate(&self) -> Result<()> {
        // 暂时跳过迁移，可以后续添加实际的迁移文件
        Ok(())
    }
    
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}