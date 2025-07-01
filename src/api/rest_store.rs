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
use std::sync::Arc;                     // 线程安全共享指针
use uuid::Uuid;                         // 生成唯一ID

use crate::container::rest_store::Container;

// #region 相关类型

/// 存储项
/// - `id` 唯一标识符 (uuid或其他字符串，一般前者配合hashmap会更好，字符串长度应限制?)
/// - `data` 事项内容 (可以是任意json项(object/string/...))
#[derive(Debug, Serialize, Clone)]
struct Item {
    id: String,
    data: Value,
}
type ItemContainer = Arc<Container<Item>>;

const API_ROOT_STR: &str = "rest/";

// #endregion

/// 创建 RESTful API 路由
pub async fn factory_rest_router() -> Router {
    let data = Container::<Item>::new_arc();

    // axum
    let app = Router::new()
        .route("/rest", get(rest_id_get).put(rest_id_put).post(rest_id_post).delete(rest_id_delete))
        .route("/rest/{id}", get(rest_id_get).put(rest_id_put).post(rest_id_post).patch(rest_id_patch).delete(rest_id_delete))
        .with_state(data); // 注入共享状态（数据库）
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
    pagination: Query<GetPagination>,
    State(data): State<ItemContainer>,
) -> impl IntoResponse {
    match id {
        // 有id，则查找特定ID项
        Some(Path(id)) => {
            tracing::debug!("GET /{}{}", API_ROOT_STR, id); // TODO 用统一的中间件来处理
            data.get_by_id(&id)
                .map_or_else(
                    || StatusCode::NOT_FOUND.into_response(),
                    |result| Json(result.clone()).into_response()
                )
        }
        // 无id，返回所有项
        None => {
            tracing::debug!("GET /{}", API_ROOT_STR);
            let result: Vec<Item> = data.get_all()
                .values()
                .skip(pagination.offset.unwrap_or(0))
                .take(pagination.limit.unwrap_or(usize::MAX))
                .cloned()
                .collect::<Vec<_>>();
            Json(result).into_response()
        }
    }
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
    State(data): State<ItemContainer>,
    Json(input): Json<RequestType>,
) -> impl IntoResponse {
    let id = id
        .map_or_else(
            || {
                let id = Uuid::new_v4().to_string();
                tracing::debug!("PUT /{}, create id:{}", API_ROOT_STR, id);
                id
            },
            |p| {
                tracing::debug!("PUT /{}{}", API_ROOT_STR, p.0);
                p.0
            }
        );

    let item = Item {
        id: id.clone(),
        data: input.data.unwrap_or(Value::Null),
    };
    
    data.put_by_id(&id, item.clone());
    (StatusCode::CREATED, Json(item))
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
    State(data): State<ItemContainer>,
    Json(input): Json<RequestType>,
) -> impl IntoResponse {
    let id = id
        .map_or_else(
            || {
                let id = Uuid::new_v4().to_string();
                tracing::debug!("POST /{}, create id:{}", API_ROOT_STR, id);
                id
            },
            |p| {
                tracing::debug!("POST /{}{}", API_ROOT_STR, p.0);
                p.0
            }
        );

    data.get_by_id(&id)
        .map_or_else(
            || {
                let item = Item {
                    id: id.clone(),
                    data: input.data.unwrap_or(Value::Null),
                };
                data.put_by_id(&id, item.clone());
                (StatusCode::CREATED, Json(item))
            },
            |result| (StatusCode::CONFLICT, Json(result))
        )
}

/**
 * PATCH /rest/{id} 更新项 (缺失策略: 404, 而非新建)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn rest_id_patch(
    Path(id): Path<String>,
    State(data): State<ItemContainer>,
    Json(input): Json<RequestType>,
) -> impl IntoResponse {
    tracing::debug!("PATCH /{}{}", API_ROOT_STR, id);

    let old_value = data.get_by_id(&id);
    if old_value.is_none() {
        return StatusCode::NOT_FOUND.into_response()
    };

    let new_value = Item {
        id: id.clone(),
        data: input.data.unwrap_or(Value::default())
    };

    data.put_by_id(&id, new_value.clone());
    Json(new_value).into_response()
}

/**
 * DELETE /rest/{id} 删除待办事项
 * 
 * - `id` 路径中的ID
 * - `db` 共享数据库状态
 */
async fn rest_id_delete(
    id: Option<Path<String>>,
    State(data): State<ItemContainer>,
) -> impl IntoResponse {
    let id = if let Some(id) = id {
        tracing::debug!("DELETE /{}{}", API_ROOT_STR, id.0);
        id.0
    } else {
        tracing::warn!("DELETE /{}, clearing is a high-risk operation", API_ROOT_STR);
        // data._delete_all();
        return StatusCode::FORBIDDEN.into_response();
    };

    let result = data.delete_by_id(&id);
    match result {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// #region api struct

#[derive(Debug, Deserialize, Default)]
struct GetPagination {
    /// 起始位置
    offset: Option<usize>,
    /// 数量限制
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RequestType {
    data: Option<Value>,
}

// #endregion
