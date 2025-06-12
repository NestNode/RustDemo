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

// 待办事项数据结构
#[derive(Debug, Serialize, Clone)]
struct Rest {
    id: Uuid,           // 唯一标识符
    data: String,       // 事项内容 (可以是json字符串)
}

type Db = Arc<RwLock<HashMap<Uuid, Rest>>>; // 数据库：内存存储，线程安全的HashMap，使用读写锁保护

// 主异步函数，使用tokio运行时
pub async fn factory_rest_router() -> Router {
    // 创建内存数据库（使用读写锁保护的HashMap）
    let db = Db::default();

    // 构建路由和中间件
    let app = Router::new()
        .route("/rest", get(rest_id_get).post(rest_id_post))
        .route("/rest/{id}", get(rest_id_get).post(rest_id_post).patch(rest_patch).delete(rest_delete))
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

// GET /rest/{id?} 获取待办事项
async fn rest_id_get(
    id: Option<Path<Uuid>>,       // 路径中的ID (可选, 无则获取全部)
    pagination: Query<RestGetPagination>,// 查询参数
    State(db): State<Db>          // 共享数据库状态
) -> impl IntoResponse {
    // 获取读锁访问数据库
    let rest = db.read().unwrap();

    match id {
        Some(Path(id)) => {
            tracing::debug!("GET /rest/{}", id);
            
            // 查找特定ID的待办事项
            match rest.get(&id) {
                Some(rest) => {
                    Json(rest.clone()).into_response()
                },
                None => {
                    StatusCode::NOT_FOUND.into_response()
                }
            }
        }
        None => {
            tracing::debug!("GET /rest/");

            // 应用分页逻辑
            let rest = rest
                .values()
                .skip(pagination.offset.unwrap_or(0))         // 跳过指定偏移量
                .take(pagination.limit.unwrap_or(usize::MAX)) // 限制返回数量
                .cloned()                 // 克隆数据
                .collect::<Vec<_>>();     // 收集为Vec

            // 返回JSON格式的待办事项列表
            Json(rest).into_response()
        }
    }
}
#[derive(Debug, Deserialize, Default)]
struct RestGetPagination {
    offset: Option<usize>,       // 起始位置
    limit: Option<usize>,        // 返回数量限制
}

// POST /rest/{id?} 创建新待办事项 TODO 检查是否已存在
async fn rest_id_post(
    id: Option<Path<Uuid>>,       // 路径中的ID (可选, 无则随机id)
    State(db): State<Db>,         // 共享数据库状态
    Json(input): Json<RestPostJson> // JSON请求体
) -> impl IntoResponse {
    let id = id.map(|p| p.0).unwrap_or_else(Uuid::new_v4);
    tracing::debug!("POST /rest/{}", id);

    // 创建新的待办事项
    let rest = Rest {
        id: id,
        data: input.data,
    };

    // 获取写锁并插入数据库
    db.write().unwrap().insert(rest.id, rest.clone());

    // 返回201 Created状态码 (RESTful规范中表示资源已被成功创建) 和创建的待办事项
    (StatusCode::CREATED, Json(rest))
}
#[derive(Debug, Deserialize)]
struct RestPostJson {
    data: String, // 待办事项内容
}

// PATCH /rest/{id} 更新待办事项
async fn rest_patch(
    Path(id): Path<Uuid>,         // 路径中的ID
    State(db): State<Db>,         // 共享数据库状态
    Json(input): Json<RestPatchJson> // JSON请求体
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("PATCH /rest/{}", id);

    // 查找指定ID的待办事项
    let mut rest = db
        .read()
        .unwrap()
        .get(&id)
        .cloned()                 // 克隆数据
        .ok_or(StatusCode::NOT_FOUND)?; // 找不到返回404

    // 更新文本（如果提供了新文本）
    if let Some(text) = input.data {
        rest.data = text;
    }

    // 更新数据库中的待办事项
    db.write().unwrap().insert(rest.id, rest.clone());

    // 返回更新后的待办事项
    Ok(Json(rest))
}
#[derive(Debug, Deserialize)]
struct RestPatchJson {
    data: Option<String>,
}

// DELETE /rest/{id} 删除待办事项
async fn rest_delete(
    Path(id): Path<Uuid>,         // 路径中的ID
    State(db): State<Db>          // 共享数据库状态
) -> impl IntoResponse {
    tracing::debug!("DELETE /rest/{}", id);

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
