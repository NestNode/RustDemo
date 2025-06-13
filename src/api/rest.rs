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
use serde_json::Value;                  // 支持任意JSON数据
use std::{                              // 标准库
    collections::HashMap,               // 内存存储数据结构
    sync::{Arc, RwLock},                // 线程安全共享指针和读写锁
    // time::Duration,                  // 超时时间设置
};
use uuid::Uuid;                         // 生成唯一ID
// use tower::{BoxError, ServiceBuilder}; // 中间件构建工具
// use tower_http::trace::TraceLayer;   // HTTP请求追踪

/// 存储项数据结构
#[derive(Debug, Serialize, Clone)]
struct Rest {
    /// 唯一标识符 (uuid或其他字符串，一般前者配合hashmap会更好，字符串长度应限制?)
    id: String,
    /// 事项内容 (可以是任意json项(object/string/...))
    data: Value,
}
/// 数据库。`原子计数(多线程多所有权安全)<读写锁<HashMap(内存存储)>>`
type Db = Arc<RwLock<HashMap<String, Rest>>>;

#[derive(Debug, Deserialize)]
struct RestRequest {
    data: Option<Value>,
}

/// 创建 RESTful API 路由
pub async fn factory_rest_router() -> Router {
    let db = Db::default();

    // axum
    let app = Router::new()
        .route("/rest", get(rest_id_get).post(rest_id_post))
        .route("/rest/{id}", get(rest_id_get).put(rest_id_put).post(rest_id_post).patch(rest_patch).delete(rest_delete))
        .with_state(db); // 注入共享状态（数据库）
    app
}

/**
 * GET /rest/{id?} 获取项
 * 
 * - `id` 路径中的ID (可选, 无则获取全部)
 * - `pagination` 查询参数
 * - `db` 共享数据库状态
 */
async fn rest_id_get(
    id: Option<Path<String>>,
    pagination: Query<RestGetPagination>,
    State(db): State<Db>
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
    /// 起始位置
    offset: Option<usize>,
    /// 数量限制
    limit: Option<usize>,
}

/**
 * PUT /rest/{id?} 幂等创建/修改项 (重复策略：覆盖，而非报错)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn rest_id_put(
    id: Option<Path<String>>,
    State(db): State<Db>,
    Json(input): Json<RestRequest>
) -> impl IntoResponse {
    let id = id.map(|p| p.0).unwrap_or_else(|| Uuid::new_v4().to_string());
    tracing::debug!("PUT /rest/{}", id);

    let rest = Rest {
        id: id,
        data: input.data.unwrap_or(Value::Null),
    };
    db.write().unwrap().insert(rest.id.clone(), rest.clone());

    (StatusCode::CREATED, Json(rest))
}

/**
 * POST /rest/{id?} 创建新项 (重复策略：409)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn rest_id_post(
    id: Option<Path<String>>,
    State(db): State<Db>,
    Json(input): Json<RestRequest>
) -> impl IntoResponse {
    let id = id.map(|p| p.0).unwrap_or_else(|| Uuid::new_v4().to_string());
    tracing::debug!("POST /rest/{}", id);

    let rest = db
        .read()
        .unwrap()
        .get(&id)
        .cloned();

    match rest {
        Some(rest) => {
            (StatusCode::CONFLICT, Json(rest))
        },
        None => {
            let rest = Rest {
                id: id,
                data: input.data.unwrap_or(Value::Null),
            };
            db.write().unwrap().insert(rest.id.clone(), rest.clone());
        
            (StatusCode::CREATED, Json(rest))
        }
    }
}

/**
 * PATCH /rest/{id} 更新项 (缺失策略: 404, 而非新建)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn rest_patch(
    Path(id): Path<String>,
    State(db): State<Db>,
    Json(input): Json<RestRequest>
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("PATCH /rest/{}", id);

    let mut rest = db
        .read()
        .unwrap()
        .get(&id)
        .cloned() // 克隆数据
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(val) = input.data {
        rest.data = val;
    }
    db.write().unwrap().insert(rest.id.clone(), rest.clone());

    Ok(Json(rest))
}

/**
 * DELETE /rest/{id} 删除待办事项
 * 
 * - `id` 路径中的ID
 * - `db` 共享数据库状态
 */
async fn rest_delete(
    Path(id): Path<String>,           
    State(db): State<Db>            
) -> impl IntoResponse {
    tracing::debug!("DELETE /rest/{}", id);

    if db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
