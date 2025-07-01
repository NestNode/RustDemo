//! (开发中)
//! 
//! 通过api在后端创建Node对象 (可以创建在一个数组/字典里)
//! 
//! 可被创建的对象遵循一些模板与特征:
//! 
//! - 均为 `Node` 的派生类 (符合Node特征)
//! - 生命周期: 归创建者所有，删除用户则会消除属于该创建者的所有对象
//! - 几个重要成员:
//!   - `id`
//!   - `next_id/next_obj` / `prev_id/prev_obj` (可能是数组)
//!   - `script` 可能是如果是脚本型 (lua/python等)，不过这需要相应的后端环境

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

// #region Node相关类型

/// Node特征
/// 
/// 必须实现线程安全约束
trait Node: Send + Sync {
    /// 依次执行脚本 (执行自身，并自动调动下一个节点)
    fn _run(&self) -> bool;

    /// 创建Node的派生类
    /// 
    /// - 自动分发类型
    fn factory(id: &str, data: Option<Value>) -> BasicNode {
        match data {
            Some(value) => {
                BasicNode {
                    id: id.to_string(),
                    content: value,
                    next_id: None,
                    prev_id: None,
                }
            },
            None => {
                BasicNode {
                    id: id.to_string(),
                    content: Value::default(),
                    next_id: None,
                    prev_id: None,
                }
            }
        }
    }

    /// factory() 的自动管理容器的版本
    fn factory_put(container:ItemContainer, id: &str, data: Option<Value>) -> BasicNode {
        let new_value = Item::factory(&id, data);

        container.put_by_id(&id, new_value.clone());
        new_value
    }

    /// factory() 的自动管理容器的版本
    fn factory_post(container:ItemContainer, id: &str, data: Option<Value>) -> (bool, BasicNode) {
        let old_value = container.get_by_id(&id);
        if let Some(value) = old_value {
            return (false, value);
        }

        let new_value = Item::factory(&id, data);

        container.put_by_id(&id, new_value.clone());
        (true, new_value)
    }
}

/// 基础节点结构体，实现Node trait
/// 
/// 存储项
#[derive(Debug, Serialize, Clone)]
struct BasicNode {
    id: String,
    content: Value, // type(预设)/运行脚本，或指向对应的对象
    next_id: Option<String>,
    prev_id: Option<String>,
}

impl Node for BasicNode {
    fn _run(&self) -> bool {
        false
    }
}

type Item = BasicNode;
type ItemContainer = Arc<Container<Item>>;

const API_ROOT_STR: &str = "node/";

// #endregion

/// 创建 Node API 路由
pub async fn factory_node_router() -> Router {
    let data = Container::<Item>::new_arc();

    // axum
    let app = Router::new()
        .route("/node", get(node_id_get).put(node_id_put).post(node_id_post))
        .route("/node/{id}", get(node_id_get).put(node_id_put).post(node_id_post).patch(node_id_patch).delete(node_id_delete))
        .with_state(data); // 注入共享状态（节点存储）
    app
}

// #region 具体路由

/**
 * GET /node/{id?} 获取项
 * 
 * - `id` 路径中的ID (可选, 无则获取全部)
 * - `pagination` 查询参数
 * - `db` 共享数据库状态
 */
async fn node_id_get(
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
 * PUT /node/{id?} 幂等创建/修改项 (重复策略：覆盖，而非报错)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn node_id_put(
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

    let item = Item::factory_put(data, &id, input.data);
    (StatusCode::CREATED, Json(item.clone()))
}

/**
 * POST /node/{id?} 创建新项 (重复策略：409)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn node_id_post(
    id: Option<Path<String>>,
    State(data): State<ItemContainer>,
    Json(input): Json<RequestType>
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

    let item = Item::factory_post(data, &id, input.data);
    if item.0 == false {
        (StatusCode::CONFLICT, Json(item.1.clone()))
    } else {
        (StatusCode::CREATED, Json(item.1.clone()))
    }
}

/**
 * PATCH /node/{id} 更新项 (缺失策略: 404, 而非新建)
 * 
 * - `id` 路径中的ID (可选, 无则随机id)
 * - `db` 共享数据库状态
 * - `input` JSON请求体
 */
async fn node_id_patch(
    Path(id): Path<String>,
    State(data): State<ItemContainer>,
    Json(input): Json<RequestType>,
) -> impl IntoResponse {
    tracing::debug!("PATCH /{}{}", API_ROOT_STR, id);

    let old_value = data.get_by_id(&id);
    if old_value.is_none() {
        return StatusCode::NOT_FOUND.into_response()
    };

    let new_value = Item::factory_put(data, &id, input.data);
    Json(new_value).into_response()
}

/**
 * DELETE /node/{id} 删除待办事项
 * 
 * - `id` 路径中的ID
 * - `db` 共享数据库状态
 */
async fn node_id_delete(
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
    data: Option<Value>,
}

// #endregion
