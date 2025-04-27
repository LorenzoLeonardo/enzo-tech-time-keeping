use axum::{
    Extension,
    body::Body,
    http::Request,
    response::{Html, IntoResponse},
};
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

use crate::{db::Db, utils};

const ITEMS_PER_PAGE: u64 = 20;
const HTML_PATH: &str = "www/*.html";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Parameters {
    user_id: String,
    #[serde(deserialize_with = "normalize_is_admin")]
    is_admin: bool,
    name: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    #[serde(deserialize_with = "normalize_page")]
    page: Option<u64>,
}

fn normalize_page<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let page: Option<String> = Option::deserialize(deserializer)?;

    if let Some(ref p) = page {
        match p.parse::<i64>() {
            Ok(n) if n < 1 => Ok(Some(1)), // Convert negative numbers to 1
            Ok(n) => Ok(Some(n as u64)),   // Convert valid positive numbers
            Err(_) => Ok(Some(1)), // Handle invalid numbers (e.g., "abc") by defaulting to 1
        }
    } else {
        Ok(None) // If `page` is missing, return None
    }
}

fn normalize_is_admin<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let is_admin: Option<String> = Option::deserialize(deserializer)?;

    if let Some(ref v) = is_admin {
        match v.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Ok(false), // Default to false if invalid string
        }
    } else {
        Ok(false) // Default to false if missing
    }
}
pub async fn handle_timekeeping(
    Extension(mut db): Extension<Db>,
    request: Request<Body>,
) -> axum::response::Response {
    // Extract URL parameters
    let Ok(pagination) = utils::extract_url_params::<Parameters>(&request) else {
        log::error!("Failed to extract URL parameters");
        return Html("No parameters given.").into_response();
    };

    if pagination.is_admin {
        if pagination.name.is_some()
            && pagination.start_date.is_some()
            && pagination.end_date.is_some()
        {
            render_for_admins_with_filter(&pagination, &mut db).await
        } else {
            render_for_admins(&pagination, &mut db).await
        }
    } else {
        render_for_none_admins(&pagination, &mut db).await
    }
}

async fn render_for_admins(pagination: &Parameters, db: &mut Db) -> axum::response::Response {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = ITEMS_PER_PAGE;
    let tera = Tera::new(HTML_PATH).unwrap();
    let total_users = db.device_login().size().await;
    let total_pages = (total_users as f64 / per_page as f64).ceil() as u64;
    let page = page.min(total_pages.max(1));
    let mut users = db.device_login().device_login_history(per_page, page).await;

    users.iter_mut().for_each(|user| {
        let utc_time = user.created_at.parse::<DateTime<Utc>>().unwrap();
        let gmt_plus_8 = FixedOffset::east_opt(8 * 3600).expect("Invalid offset");
        let local_time = utc_time.with_timezone(&gmt_plus_8);
        let created_at = local_time.format("%Y-%m-%d %H:%M:%S%.3f %:z").to_string();

        user.created_at = created_at
    });

    let mut context = Context::new();
    context.insert("users", &users);
    context.insert("current_page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("next_page", &(page + 1));
    context.insert("prev_page", &(page.saturating_sub(1)));
    context.insert("name", &"");
    context.insert("start_date", &"");
    context.insert("end_date", &"");
    context.insert("is_admin", &true);
    context.insert("user_id", &pagination.user_id);

    let rendered = tera.render("timekeeping.html", &context).unwrap();
    Html(rendered).into_response()
}

async fn render_for_none_admins(pagination: &Parameters, db: &mut Db) -> axum::response::Response {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = ITEMS_PER_PAGE;
    let tera = Tera::new(HTML_PATH).unwrap();
    let total_users = db
        .device_login()
        .size_per_user(pagination.user_id.as_str())
        .await;
    let total_pages = (total_users as f64 / per_page as f64).ceil() as u64;
    let page = page.min(total_pages.max(1));
    let mut users = db
        .device_login()
        .device_login_history_per_user(pagination.user_id.as_str(), per_page, page)
        .await;

    users.iter_mut().for_each(|user| {
        let utc_time = user.created_at.parse::<DateTime<Utc>>().unwrap();
        let gmt_plus_8 = FixedOffset::east_opt(8 * 3600).expect("Invalid offset");
        let local_time = utc_time.with_timezone(&gmt_plus_8);
        let created_at = local_time.format("%Y-%m-%d %H:%M:%S%.3f %:z").to_string();

        user.created_at = created_at
    });

    let mut context = Context::new();
    context.insert("users", &users);
    context.insert("current_page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("next_page", &(page + 1));
    context.insert("prev_page", &(page.saturating_sub(1)));
    context.insert("name", &"");
    context.insert("start_date", &"");
    context.insert("end_date", &"");
    context.insert("is_admin", &false);
    context.insert("user_id", &pagination.user_id);

    let rendered = tera.render("timekeeping.html", &context).unwrap();
    Html(rendered).into_response()
}

async fn render_for_admins_with_filter(
    pagination: &Parameters,
    db: &mut Db,
) -> axum::response::Response {
    let (Some(name), Some(start_date), Some(end_date)) = (
        pagination.name.clone(),
        pagination.start_date.clone(),
        pagination.end_date.clone(),
    ) else {
        log::error!("Invalid input");
        return Html("Invalid input").into_response();
    };
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = ITEMS_PER_PAGE;
    let tera = Tera::new(HTML_PATH).unwrap();
    let total_users = db
        .device_login()
        .admin_filter_login_status_by_name_and_date_count(
            name.as_str(),
            start_date.as_str(),
            end_date.as_str(),
        )
        .await;
    let total_pages = (total_users as f64 / per_page as f64).ceil() as u64;
    let page = page.min(total_pages.max(1));
    let mut users = db
        .device_login()
        .admin_filter_login_status_by_name_and_date(
            per_page,
            page,
            name.as_str(),
            start_date.as_str(),
            end_date.as_str(),
        )
        .await;

    users.iter_mut().for_each(|user| {
        let utc_time = user.created_at.parse::<DateTime<Utc>>().unwrap();
        let gmt_plus_8 = FixedOffset::east_opt(8 * 3600).expect("Invalid offset");
        let local_time = utc_time.with_timezone(&gmt_plus_8);
        let created_at = local_time.format("%Y-%m-%d %H:%M:%S%.3f %:z").to_string();

        user.created_at = created_at
    });

    let mut context = Context::new();
    context.insert("users", &users);
    context.insert("current_page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("next_page", &(page + 1));
    context.insert("prev_page", &(page.saturating_sub(1)));
    context.insert("name", &name);
    context.insert("start_date", &start_date);
    context.insert("end_date", &end_date);
    context.insert("is_admin", &true);
    context.insert("user_id", &pagination.user_id);

    let rendered = tera.render("timekeeping.html", &context).unwrap();
    Html(rendered).into_response()
}

#[allow(dead_code)]
async fn render_for_none_admins_with_filter(
    pagination: &Parameters,
    db: &mut Db,
) -> axum::response::Response {
    let (Some(name), Some(start_date), Some(end_date)) = (
        pagination.name.clone(),
        pagination.start_date.clone(),
        pagination.end_date.clone(),
    ) else {
        log::error!("Invalid input");
        return Html("Invalid input").into_response();
    };
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = ITEMS_PER_PAGE;
    let tera = Tera::new("www/*.html").unwrap();
    let total_users = db
        .device_login()
        .none_admin_filter_login_status_by_name_and_date_count(
            pagination.user_id.as_str(),
            start_date.as_str(),
            end_date.as_str(),
        )
        .await;
    let total_pages = (total_users as f64 / per_page as f64).ceil() as u64;
    let page = page.min(total_pages.max(1));
    let mut users = db
        .device_login()
        .none_admin_filter_login_status_by_name_and_date(
            pagination.user_id.as_str(),
            per_page,
            page,
            name.as_str(),
            start_date.as_str(),
            end_date.as_str(),
        )
        .await;

    users.iter_mut().for_each(|user| {
        let utc_time = user.created_at.parse::<DateTime<Utc>>().unwrap();
        let gmt_plus_8 = FixedOffset::east_opt(8 * 3600).expect("Invalid offset");
        let local_time = utc_time.with_timezone(&gmt_plus_8);
        let created_at = local_time.format("%Y-%m-%d %H:%M:%S%.3f %:z").to_string();

        user.created_at = created_at
    });

    let mut context = Context::new();
    context.insert("users", &users);
    context.insert("current_page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &total_pages);
    context.insert("next_page", &(page + 1));
    context.insert("prev_page", &(page.saturating_sub(1)));
    context.insert("name", &name);
    context.insert("start_date", &start_date);
    context.insert("end_date", &end_date);
    context.insert("is_admin", &false);
    context.insert("user_id", &pagination.user_id);

    let rendered = tera.render("timekeeping.html", &context).unwrap();
    Html(rendered).into_response()
}
