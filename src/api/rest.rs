//! 通用的 RESTful API
//!
//! (分单/多人版本，多人版本会需要秘钥、隔离。目前仅支持单人版本。
//! 不管哪个版本，api都是通用的，多人版本还需要auth/cookies)
//!
//! API接口设计：
//!
//! - `GET /rest`: 返回所有存储项
//! - `POST /rest`: 创建新的存储项
//! - `PATCH /rest/{id}`: 更新指定ID的存储项
//! - `DELETE /rest/{id}`: 删除指定ID的存储项

use axum::{
    // error_handling::HandleErrorLayer,// 错误处理中间件
    extract::{Path, Query, State},      // 请求提取器（路径参数、查询参数、状态）
    http::StatusCode,                   // HTTP状态码
    response::IntoResponse,             // 响应转换trait
    routing::{get},                     // HTTP方法路由
    Json, Router,                       // JSON处理、路由器
};
use serde::{Deserialize, Serialize};    // JSON序列化/反序列化
use std::{                              // 标准库
    collections::HashMap,               // 内存存储数据结构
    sync::{Arc, RwLock},                // 线程安全共享指针和读写锁
    // time::Duration,                  // 超时时间设置
};
use uuid::Uuid;                         // 生成唯一ID
// use tower::{BoxError, ServiceBuilder}; // 中间件构建工具
// use tower_http::trace::TraceLayer;   // HTTP请求追踪

// 存储项数据结构
#[derive(Debug, Serialize, Clone)]
struct Rest {
    id: Uuid,           // 唯一标识符
    data: String,       // 事项内容 (可以是json字符串)
}
#[derive(Debug, Deserialize)]
struct RestRequest {
    data: Option<String>,
}

type Db = Arc<RwLock<HashMap<Uuid, Rest>>>; // 数据库：内存存储，线程安全的HashMap，使用读写锁保护

pub async fn factory_rest_router() -> Router {
    // 创建内存数据库（使用读写锁保护的HashMap）
    let db = Db::default();

    // 构建路由和中间件
    let app = Router::new()
        .route("/rest", get(rest_id_get).post(rest_id_post))
        .route("/rest/{id}", get(rest_id_get).post(rest_id_post).patch(rest_patch).delete(rest_delete))
        .with_state(db); // 注入共享状态（数据库）
    app
}

// GET /rest/{id?} 获取项
async fn rest_id_get(
    id: Option<Path<Uuid>>,       // 路径中的ID (可选, 无则获取全部)
    pagination: Query<RestGetPagination>,// 查询参数
    State(db): State<Db>          // 共享数据库状态
) -> impl IntoResponse {
    match id {
        // 有id，则查找特定ID项
        Some(Path(id)) => {
            tracing::debug!("GET /rest/{}", id);
            let rest = db.read().unwrap();
            match rest.get(&id) {
                Some(rest) => {
                    Json(rest.clone()).into_response()
                },
                None => {
                    StatusCode::NOT_FOUND.into_response()
                }
            }
        }
        // 无id，返回所有项
        None => {
            tracing::debug!("GET /rest/");
            let rest = db.read().unwrap();
            let rest = rest
                .values()
                .skip(pagination.offset.unwrap_or(0))           // 跳过指定偏移量
                .take(pagination.limit.unwrap_or(usize::MAX))   // 限制返回数量
                .cloned()                                       // 克隆数据
                .collect::<Vec<_>>();                           // 收集为Vec
            Json(rest).into_response()
        }
    }
}
#[derive(Debug, Deserialize, Default)]
struct RestGetPagination {
    offset: Option<usize>,       // 起始位置
    limit: Option<usize>,        // 数量限制
}

// POST /rest/{id?} 创建新项 (重复策略：覆盖，而非报错)
async fn rest_id_post(
    id: Option<Path<Uuid>>,       // 路径中的ID (可选, 无则随机id)
    State(db): State<Db>,         // 共享数据库状态
    Json(input): Json<RestRequest> // JSON请求体
) -> impl IntoResponse {
    let id = id.map(|p| p.0).unwrap_or_else(Uuid::new_v4);
    tracing::debug!("POST /rest/{}", id);

    // 写入新项
    let rest = Rest {
        id: id,
        data: input.data.unwrap_or_else(String::new),
    };
    db.write().unwrap().insert(rest.id, rest.clone());

    (StatusCode::CREATED, Json(rest)) // 201 (Created状态码) 和新项
}

// PATCH /rest/{id} 更新项 (缺失策略: 404, 而非新建)
async fn rest_patch(
    Path(id): Path<Uuid>,           // 路径中的ID
    State(db): State<Db>,           // 共享数据库状态
    Json(input): Json<RestRequest>  // JSON请求体
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("PATCH /rest/{}", id);

    // 查找项
    let mut rest = db
        .read()
        .unwrap()
        .get(&id)
        .cloned()                   // 克隆数据
        .ok_or(StatusCode::NOT_FOUND)?; // 找不到返回404

    // 更新项
    if let Some(text) = input.data {
        rest.data = text;
    }
    db.write().unwrap().insert(rest.id, rest.clone());

    Ok(Json(rest))
}

// DELETE /rest/{id} 删除存储项
async fn rest_delete(
    Path(id): Path<Uuid>,           // 路径中的ID
    State(db): State<Db>            // 共享数据库状态
) -> impl IntoResponse {
    tracing::debug!("DELETE /rest/{}", id);

    // 删除指定ID项
    if db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT      // 204 (No Content) 表示删除成功但无需返回内容
    } else {
        StatusCode::NOT_FOUND       // 404 (Not Found) 表示找不到
    }
}
