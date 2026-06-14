use crate::constants::API_UNAVAILABLE_MESSAGE;
use gloo_net::http::Request;
use serde::Deserialize;
use serde_json::Value;

mod audit;
mod bootstrap;
mod cases;
mod data_sources;
mod evidence;
mod governance;
mod medical;
mod models;
mod qa;
mod rules;
mod scoring;
mod summaries;

pub(crate) use audit::*;
pub(crate) use bootstrap::*;
pub(crate) use cases::*;
pub(crate) use data_sources::*;
pub(crate) use evidence::*;
pub(crate) use governance::*;
pub(crate) use medical::*;
pub(crate) use models::*;
pub(crate) use qa::*;
pub(crate) use rules::*;
pub(crate) use scoring::*;
pub(crate) use summaries::*;

pub(crate) async fn request_json<T>(
    path: &str,
    api_key: String,
    payload: Value,
) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let request = Request::post(path)
        .header("content-type", "application/json")
        .header("x-api-key", &api_key)
        .body(payload.to_string())
        .map_err(|error| error.to_string())?;
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    parse_json_response(path, status, &body)
}

pub(crate) async fn request_get_json<T>(path: &str, api_key: String) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = Request::get(path)
        .header("x-api-key", &api_key)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    parse_json_response(path, status, &body)
}

fn parse_json_response<T>(path: &str, status: u16, body: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let body = body.trim();
    if !(200..300).contains(&status) {
        return Err(api_error_message(path, status, body));
    }
    if body.is_empty() {
        return Err(API_UNAVAILABLE_MESSAGE.to_string());
    }
    let body: Value = serde_json::from_str(body)
        .map_err(|error| format!("Invalid API response from {path}: {error}"))?;
    serde_json::from_value(body).map_err(|error| error.to_string())
}

fn api_error_message(path: &str, status: u16, body: &str) -> String {
    if body.is_empty() {
        return API_UNAVAILABLE_MESSAGE.to_string();
    }
    match serde_json::from_str::<Value>(body) {
        Ok(body) => body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}: {}", pretty_json(&body))),
        Err(_) => format!("HTTP {status} from {path}: {body}"),
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
