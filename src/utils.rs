use std::collections::HashMap;

use axum::{body::Body, http::Request};
use serde::de::DeserializeOwned;
use url::form_urlencoded;

pub fn extract_url_params<T>(req: &Request<Body>) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let url = req.uri();

    let query = url
        .query()
        .ok_or_else(|| "Failed to extract query string from request".to_string())?;

    let parsed: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    log::info!("Parsed URL parameters: {parsed:?}");
    let json_string = serde_json::to_string(&parsed).map_err(|e| {
        log::error!("{e}");
        e.to_string()
    })?;

    serde_json::from_str::<T>(&json_string).map_err(|e| {
        log::error!("{e}");
        e.to_string()
    })
}
