//! 管理待办事项 (Todos) 的 RESTful API
//!
//! API接口设计：
//!
//! - `GET /todos`: 返回所有待办事项的JSON列表
//! - `POST /todos`: 创建新的待办事项
//! - `PATCH /todos/{id}`: 更新指定ID的待办事项
//! - `DELETE /todos/{id}`: 删除指定ID的待办事项

use axum::{
    // error_handling::HandleErrorLayer,// 错误处理中间件
    extract::{Path, Query, State},      // 请求提取器（路径参数、查询参数、状态）
    http::StatusCode,                   // HTTP状态码
    response::{IntoResponse},           // 响应转换trait
    routing::{get},                     // HTTP方法路由
    Json, Router,                       // JSON处理、路由器
};
use serde::{Deserialize, Serialize};    // JSON序列化/反序列化
use std::sync::Arc;                     // 线程安全共享指针
use uuid::Uuid;                         // 生成唯一ID

use crate::container::rest_store::Container;

// #region 相关类型

/// 存储项 (待办事项)
/// - `id` 唯一标识符 (uuid或其他字符串，一般前者配合hashmap会更好，字符串长度应限制?)
/// - `data` 事项内容
/// - `completed` 完成状态
#[derive(Debug, Serialize, Clone)]
struct Item {
    id: String,
    text: String,
    completed: bool,
}
type ItemContainer = Arc<Container<Item>>;

const API_ROOT_STR: &str = "todos/";

// #endregion

/// 创建 RESTful API 路由
pub async fn factory_todos_router() -> Router {
    let data = Container::<Item>::new_arc();

    // axum
    let app = Router::new()
        .route("/todos", get(todos_id_get).put(todos_id_put).post(todos_id_post))
        .route("/todos/{id}", get(todos_id_get).put(todos_id_put).post(todos_id_post).patch(todos_id_patch).delete(todos_id_delete))
        .with_state(data); // 注入共享状态（数据库）
    app
}

/**
 * GET /todos/{id?} 获取项
 * 
 * - `id` 路径中的ID (可选, 无则获取全部)
 * - `pagination` 查询参数
 * - `db` 共享数据库状态
 */
async fn todos_id_get(
    id: Option<Path<String>>,
    pagination: Query<GetPagination>, 
    State(data): State<ItemContainer>
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
 * PUT /todos/{id?} 幂等创建/修改项 (重复策略：覆盖，而非报错)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn todos_id_put(
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
        text: input.text.unwrap_or(String::new()),
        completed: input.completed.unwrap_or(false),
    };
    
    data.put_by_id(&id, item.clone());
    (StatusCode::CREATED, Json(item))
}

/**
 * POST /todos/{id?} 创建新项 (重复策略：409)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn todos_id_post(
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
                    text: input.text.unwrap_or(String::new()),
                    completed: input.completed.unwrap_or(false),
                };
                data.put_by_id(&id, item.clone());
                (StatusCode::CREATED, Json(item))
            },
            |result| (StatusCode::CONFLICT, Json(result))
        )
}

/**
 * PATCH /todos/{id} 更新项 (缺失策略: 404, 而非新建)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn todos_id_patch(
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
        text: input.text.unwrap_or(String::default()),
        completed: input.completed.unwrap_or(false)
    };

    data.put_by_id(&id, new_value.clone());
    Json(new_value).into_response()
}

/**
 * DELETE /todos/{id} 删除待办事项
 * 
 * - `id` 路径中的ID
 * - `db` 共享数据库状态
 */
async fn todos_id_delete (
    Path(id): Path<String>,           
    State(data): State<ItemContainer>,
) -> impl IntoResponse {
    tracing::debug!("DELETE /{}{}", API_ROOT_STR, id);

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
    text: Option<String>,
    completed: Option<bool>,
}

// #endregion
