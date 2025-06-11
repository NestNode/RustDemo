//! 提供一个管理待办事项(Todos)的RESTful Web服务器。
//!
//! API接口设计：
//!
//! - `GET /todos`: 返回所有待办事项的JSON列表
//! - `POST /todos`: 创建新的待办事项
//! - `PATCH /todos/{id}`: 更新指定ID的待办事项
//! - `DELETE /todos/{id}`: 删除指定ID的待办事项
//!
//! 运行命令：
//!
//! ```sh
//! cargo run -p example-todos
//! ```

use axum::{
    // error_handling::HandleErrorLayer,  // 错误处理中间件
    extract::{Path, Query, State},     // 请求提取器（路径参数、查询参数、状态）
    http::StatusCode,                  // HTTP状态码
    response::IntoResponse,            // 响应转换trait
    routing::{get, patch},             // HTTP方法路由
    Json, Router,                      // JSON处理、路由器
};
use serde::{Deserialize, Serialize};   // JSON序列化/反序列化
use std::{                             // 标准库
    collections::HashMap,              // 内存存储数据结构
    sync::{Arc, RwLock},               // 线程安全共享指针和读写锁
    // time::Duration,                    // 超时时间设置
};
use uuid::Uuid;                        // 生成唯一ID
// use tower::{BoxError, ServiceBuilder}; // 中间件构建工具
// use tower_http::trace::TraceLayer;     // HTTP请求追踪

// 待办事项数据结构
#[derive(Debug, Serialize, Clone)]
struct Todo {
    id: Uuid,           // 唯一标识符
    text: String,       // 事项内容
    completed: bool,    // 完成状态
}

type Db = Arc<RwLock<HashMap<Uuid, Todo>>>; // 数据库：内存存储，线程安全的HashMap，使用读写锁保护

// 主异步函数，使用tokio运行时
pub async fn use_todos_router() -> Router {
    // 创建内存数据库（使用读写锁保护的HashMap）
    let db = Db::default();

    // 构建路由和中间件
    let app = Router::new()
        .route("/todos", get(todos_get).post(todos_post))
        .route("/todos/{id}", patch(todos_patch).delete(todos_delete))
        // 添加全局中间件
        // .layer(
        //     ServiceBuilder::new()
        //         // 错误处理中间件：将BoxError转换为HTTP响应
        //         .layer(HandleErrorLayer::new(|error: BoxError| async move {
        //             if error.is::<tower::timeout::error::Elapsed>() {
        //                 // 请求超时处理
        //                 Ok(StatusCode::REQUEST_TIMEOUT)
        //             } else {
        //                 // 其他错误处理
        //                 Err((
        //                     StatusCode::INTERNAL_SERVER_ERROR,
        //                     format!("未处理的内部错误: {error}"),
        //                 ))
        //             }
        //         }))
        //         .timeout(Duration::from_secs(10))  // 10秒请求超时
        //         .layer(TraceLayer::new_for_http()) // HTTP请求追踪
        //         .into_inner(),
        // )
        .with_state(db); // 注入共享状态（数据库）
    app
}

// 查询参数类型
#[derive(Debug, Deserialize, Default)]
struct Pagination {
    offset: Option<usize>,       // 起始位置
    limit: Option<usize>,        // 返回数量限制
}
// GET /todos 处理函数 - 获取待办事项列表
async fn todos_get(
    pagination: Query<Pagination>,// 查询参数
    State(db): State<Db>          // 共享数据库状态
) -> impl IntoResponse {
    tracing::debug!("GET /todos/");

    // 获取读锁访问数据库
    let todos = db.read().unwrap();

    // 应用分页逻辑
    let todos = todos
        .values()
        .skip(pagination.offset.unwrap_or(0))         // 跳过指定偏移量
        .take(pagination.limit.unwrap_or(usize::MAX)) // 限制返回数量
        .cloned()                 // 克隆数据
        .collect::<Vec<_>>();     // 收集为Vec

    // 返回JSON格式的待办事项列表
    Json(todos)
}

#[derive(Debug, Deserialize)]
struct CreateTodo {
    text: String, // 待办事项内容
}
// POST /todos 处理函数 - 创建新待办事项
async fn todos_post(
    State(db): State<Db>,         // 共享数据库状态
    Json(input): Json<CreateTodo> // JSON请求体
) -> impl IntoResponse {
    tracing::debug!("POST /todos/");

    // 创建新的待办事项
    let todo = Todo {
        id: Uuid::new_v4(),       // 生成唯一ID
        text: input.text,         // 设置内容
        completed: false,         // 默认未完成
    };

    // 获取写锁并插入数据库
    db.write().unwrap().insert(todo.id, todo.clone());

    // 返回201 Created状态码和创建的待办事项
    // 201 Created 表示资源已被成功创建，符合 RESTful 规范
    (StatusCode::CREATED, Json(todo))
}

#[derive(Debug, Deserialize)]
struct UpdateTodo {
    text: Option<String>,         // 可选的新文本
    completed: Option<bool>,      // 可选的完成状态
}
// PATCH /todos/{id} 处理函数 - 更新待办事项
async fn todos_patch(
    Path(id): Path<Uuid>,         // 路径中的ID
    State(db): State<Db>,         // 共享数据库状态
    Json(input): Json<UpdateTodo> // JSON请求体
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("PATCH /todos/{{id}}");

    // 查找指定ID的待办事项
    let mut todo = db
        .read()
        .unwrap()
        .get(&id)
        .cloned()                 // 克隆数据
        .ok_or(StatusCode::NOT_FOUND)?; // 找不到返回404

    // 更新文本（如果提供了新文本）
    if let Some(text) = input.text {
        todo.text = text;
    }

    // 更新完成状态（如果提供了新状态）
    if let Some(completed) = input.completed {
        todo.completed = completed;
    }

    // 更新数据库中的待办事项
    db.write().unwrap().insert(todo.id, todo.clone());

    // 返回更新后的待办事项
    Ok(Json(todo))
}

// DELETE /todos/{id} 处理函数 - 删除待办事项
async fn todos_delete(
    Path(id): Path<Uuid>,         // 路径中的ID
    State(db): State<Db>          // 共享数据库状态
) -> impl IntoResponse {
    tracing::debug!("DELETE /todos/{{id}}");

    // 尝试删除指定ID的待办事项
    if db.write().unwrap().remove(&id).is_some() {
        // 成功删除返回204 No Content
        // 204 No Content 表示删除成功但无需返回内容，符合 RESTful 规范
        StatusCode::NO_CONTENT
    } else {
        // 找不到返回404 Not Found
        StatusCode::NOT_FOUND
    }
}
