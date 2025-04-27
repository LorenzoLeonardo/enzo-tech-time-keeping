use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Debug, FromRow, Serialize, Clone, Deserialize)]
pub struct DeviceLogin {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub device_id: String,
    pub login_status: String,
    pub ip_address: String,
    pub location: String,
    pub isp: String,
    pub created_at: String, // Assuming it's stored as TEXT (ISO8601)
}

#[derive(Clone, Debug)]
pub struct DeviceLoginDatabase {
    pool: Pool<Sqlite>,
}

impl DeviceLoginDatabase {
    pub async fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn device_login_history(
        &self,
        limit_per_page: u64,
        page_number: u64,
    ) -> Vec<DeviceLogin> {
        let limit = limit_per_page;
        let offset = limit * (page_number - 1);

        sqlx::query_as::<_, DeviceLogin>(
            r#"
            SELECT user_id, name, email, device_id,
                   login_status, ip_address, location, isp, created_at
            FROM device_login
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?;
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            log::error!("DeviceLoginDatabase::device_login_history: {err}");
            Vec::new()
        })
    }

    pub async fn device_login_history_per_user(
        &self,
        user_id: &str,
        limit_per_page: u64,
        page_number: u64,
    ) -> Vec<DeviceLogin> {
        let limit = limit_per_page;
        let offset = limit * (page_number - 1);

        sqlx::query_as::<_, DeviceLogin>(
            r#"
            SELECT user_id, name, email, device_id, 
                   login_status, ip_address, location, isp, created_at
            FROM device_login
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?;
            "#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            log::error!("DeviceLoginDatabase::device_login_history_per_user: {err}");
            Vec::new()
        })
    }

    pub async fn size_per_user(&self, user_id: &str) -> usize {
        let size: usize =
            sqlx::query_scalar::<_, u64>("SELECT COUNT(*) FROM device_login WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map(|count| count as usize)
                .unwrap_or_else(|err| {
                    log::error!("DeviceLoginDatabase::size_per_user: {err}");
                    0
                });

        size
    }

    pub async fn size(&self) -> usize {
        let size: usize = sqlx::query_scalar::<_, u64>("SELECT COUNT(*) FROM device_login")
            .fetch_one(&self.pool)
            .await
            .map(|count| count as usize)
            .unwrap_or_else(|err| {
                log::error!("DeviceLoginDatabase::size: {err}");
                0
            });

        size
    }

    pub async fn admin_filter_login_status_by_name_and_date(
        &self,
        limit_per_page: u64,
        page_number: u64,
        name_filter: &str,
        start_rfc3339: &str,
        end_rfc3339: &str,
    ) -> Vec<DeviceLogin> {
        let limit = limit_per_page;
        let offset = limit * (page_number - 1);
        sqlx::query_as::<_, DeviceLogin>(
            r#"
            SELECT user_id, name, email, device_id,
                   login_status, ip_address, location, isp, created_at
            FROM device_login
            WHERE name LIKE ?
              AND created_at BETWEEN ? AND ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?;
            "#,
        )
        .bind(format!("%{}%", name_filter))
        .bind(start_rfc3339)
        .bind(end_rfc3339)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            log::error!("DeviceLoginDatabase::admin_filter_login_status_by_name_and_date: {err}");
            Vec::new()
        })
    }

    pub async fn admin_filter_login_status_by_name_and_date_count(
        &self,
        name_filter: &str,
        start_rfc3339: &str,
        end_rfc3339: &str,
    ) -> usize {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM device_login
            WHERE name LIKE ?
              AND created_at BETWEEN ? AND ?;
            "#,
        )
        .bind(format!("%{}%", name_filter))
        .bind(start_rfc3339)
        .bind(end_rfc3339)
        .fetch_one(&self.pool)
        .await
        .map(|count| count as usize)
        .unwrap_or_else(|err| {
            log::error!(
                "DeviceLoginDatabase::admin_filter_login_status_by_name_and_date_count: {err}"
            );
            0
        })
    }

    pub async fn none_admin_filter_login_status_by_name_and_date(
        &self,
        user_id: &str,
        limit_per_page: u64,
        page_number: u64,
        name_filter: &str,
        start_rfc3339: &str,
        end_rfc3339: &str,
    ) -> Vec<DeviceLogin> {
        let limit = limit_per_page;
        let offset = limit * (page_number - 1);
        sqlx::query_as::<_, DeviceLogin>(
            r#"
            SELECT user_id, name, email, device_id,
                   login_status, ip_address, location, isp, created_at
            FROM device_login
            WHERE user_id = ?
              AND name LIKE ?
              AND created_at BETWEEN ? AND ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?;
            "#,
        )
        .bind(user_id)
        .bind(format!("%{}%", name_filter))
        .bind(start_rfc3339)
        .bind(end_rfc3339)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            log::error!("DeviceLoginDatabase::admin_filter_login_status_by_name_and_date: {err}");
            Vec::new()
        })
    }

    pub async fn none_admin_filter_login_status_by_name_and_date_count(
        &self,
        user_id: &str,
        start_rfc3339: &str,
        end_rfc3339: &str,
    ) -> usize {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM device_login
            WHERE user_id = ?
              AND created_at BETWEEN ? AND ?;
            "#,
        )
        .bind(user_id)
        .bind(start_rfc3339)
        .bind(end_rfc3339)
        .fetch_one(&self.pool)
        .await
        .map(|count| count as usize)
        .unwrap_or_else(|err| {
            log::error!(
                "DeviceLoginDatabase::none_admin_filter_login_status_by_name_and_date_count: {err}"
            );
            0
        })
    }
}
