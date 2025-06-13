//! 用于心跳检测的API

use axum::{
    http::StatusCode, response::IntoResponse, Json
};
use serde_json::json;

/// GET /heartbeat, 心脏检测
pub async fn get_heartbeat() -> impl IntoResponse {
    tracing::debug!("GET /heartbeat");

    let resp = json!({
        "status": "alive",
        "timestamp": chrono::Local::now().to_rfc3339(), // 本地时间
        // chrono::Utc::now().to_rfc3339(), // 零区
        // chrono::FixedOffset::east_opt(8 * 3600).unwrap(); // 东八区
    });

    (StatusCode::OK , Json(resp))
}
