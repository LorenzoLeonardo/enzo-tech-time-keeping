use sqlx::{Pool, Sqlite, SqlitePool};

use crate::users::device_login::DeviceLoginDatabase;

const INIT_DB: &str = r#"
CREATE TABLE IF NOT EXISTS device_login (
                    user_id TEXT NOT NULL,
                    session_id TEXT,
                    name TEXT,
                    email TEXT,
                    device_id TEXT,
                    login_provider TEXT,
                    login_status TEXT,
                    ip_address TEXT,
                    location TEXT,
                    isp TEXT,
                    created_at TEXT
                );
"#;

pub async fn init_db(db_url: &str) -> Pool<Sqlite> {
    // Create a connection pool
    let pool = SqlitePool::connect(db_url)
        .await
        .expect("Failed to connect to database");

    // Create the table if it doesn't exist
    sqlx::query(INIT_DB)
        .execute(&pool)
        .await
        .expect("Failed to create table");

    pool
}

pub async fn close_db(pool: Pool<Sqlite>) {
    pool.close().await;
}

#[derive(Clone)]
pub struct Db {
    device_login: Option<DeviceLoginDatabase>,
}

impl Db {
    pub fn new() -> Self {
        Self { device_login: None }
    }

    pub fn device_login(&mut self) -> &mut DeviceLoginDatabase {
        self.device_login.as_mut().unwrap()
    }

    pub fn set_device_login(mut self, device_login: DeviceLoginDatabase) -> Self {
        self.device_login = Some(device_login);
        self
    }
}
