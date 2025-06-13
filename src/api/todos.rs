//! 管理待办事项(Todos)的 RESTful API
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
use std::{                              // 标准库
    collections::HashMap,               // 内存存储数据结构
    sync::{Arc, RwLock},                // 线程安全共享指针和读写锁
    // time::Duration,                  // 超时时间设置
};
use uuid::Uuid;                         // 生成唯一ID
// use tower::{BoxError, ServiceBuilder}; // 中间件构建工具
// use tower_http::trace::TraceLayer;   // HTTP请求追踪

/// 待办事项数据结构
#[derive(Debug, Serialize, Clone)]
struct Todo {
    /// 唯一标识符
    id: Uuid,
    /// 事项内容
    text: String,
    /// 完成状态
    completed: bool,
}
/// 数据库。`原子计数(多线程多所有权安全)<读写锁<HashMap(内存存储)>>`
type Db = Arc<RwLock<HashMap<Uuid, Todo>>>;

#[derive(Debug, Deserialize)]
struct TodosRequest {
    text: Option<String>,
    completed: Option<bool>,
}

/// 创建 RESTful API 路由
pub async fn factory_todos_router() -> Router {
    let db = Db::default();

    // axum
    let app = Router::new()
        .route("/todos", get(todos_id_get).post(todos_id_post))
        .route("/todos/{id}", get(todos_id_get).post(todos_id_post).patch(todos_id_patch).delete(todos_id_delete))
        .with_state(db); // 注入共享状态（数据库）
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
    id: Option<Path<Uuid>>,
    pagination: Query<TodosGetPagination>, 
    State(db): State<Db>
) -> impl IntoResponse {
    match id {
        // 有id，则查找特定ID项
        Some(Path(id)) => {
            tracing::debug!("GET /todos/{}", id);
            let todos = db.read().unwrap();
            match todos.get(&id) {
                Some(todo) => {
                    Json(todo.clone()).into_response()
                },
                None => {
                    StatusCode::NOT_FOUND.into_response()
                }
            }
        }
        // 无id，返回所有项
        None => {
            tracing::debug!("GET /todos/");
            let todos = db.read().unwrap();
            let todos = todos
                .values()
                .skip(pagination.offset.unwrap_or(0))           // 跳过指定偏移量
                .take(pagination.limit.unwrap_or(usize::MAX))   // 限制返回数量
                .cloned()                                       // 克隆数据
                .collect::<Vec<_>>();                           // 收集为Vec
            Json(todos).into_response()
        }
    }
}
#[derive(Debug, Deserialize, Default)]
struct TodosGetPagination {
    /// 起始位置
    offset: Option<usize>,
    /// 数量限制
    limit: Option<usize>,
}

/**
 * POST /todos/{id?} 创建新项 (重复策略：覆盖，而非报错)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn todos_id_post(
    id: Option<Path<Uuid>>,
    State(db): State<Db>,
    Json(input): Json<TodosRequest>
) -> impl IntoResponse {
    let id = id.map(|p| p.0).unwrap_or_else(Uuid::new_v4);
    tracing::debug!("POST /todos/{}", id);

    let todo = Todo {
        id: id,
        text: input.text.unwrap_or_else(String::new),
        completed: input.completed.unwrap_or(false),
    };
    db.write().unwrap().insert(todo.id, todo.clone());

    (StatusCode::CREATED, Json(todo))
}

/**
 * PATCH /todos/{id} 更新项 (缺失策略: 404, 而非新建)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn todos_id_patch(
    Path(id): Path<Uuid>,
    State(db): State<Db>,
    Json(input): Json<TodosRequest>
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("PATCH /todos/{}", id);

    let mut todo = db
        .read()
        .unwrap()
        .get(&id)
        .cloned() // 克隆数据
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(text) = input.text {
        todo.text = text;
    }
    if let Some(completed) = input.completed {
        todo.completed = completed;
    }
    db.write().unwrap().insert(todo.id, todo.clone());

    Ok(Json(todo))
}

/**
 * DELETE /todos/{id} 删除待办事项
 * 
 * - `id` 路径中的ID
 * - `db` 共享数据库状态
 */
async fn todos_id_delete (
    Path(id): Path<Uuid>,
    State(db): State<Db>
) -> impl IntoResponse {
    tracing::debug!("DELETE /todos/{}", id);

    if db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
