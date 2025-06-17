//! 用于心跳检测的API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
    // extract::ConnectInfo,
    // Extension,
};
// use axum::extract::CookieJar;
use axum_extra::extract::{
    cookie::{Cookie},
    CookieJar,
};
use serde_json::{json};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::{
    collections::HashMap, sync::atomic::{AtomicU32, Ordering}, time::{Duration, Instant}
};

/// 工具路由
/// 
/// 包括心跳检测和常用工具等
pub fn factory_utils_router() -> Router {
    // 启动清理任务
    start_cleanup_task(None);

    let app = Router::new()
        .route("/heartbeat", get(get_heartbeat));
    app
}

/// 用户活跃状态结构
struct OnlineState {
    // user_activity_time: RwLock<HashMap<SocketAddr, Instant>>, // 存储用户最后活跃时间 (Ip)
    user_activity_time: RwLock<HashMap<String, Instant>>, // 存储用户最后活跃时间 (会话ID)
    user_activity_count: AtomicU32, // 原子计数器用于快速查询
}
/// 全局在线状态
/// 
/// type: 线程安全只初始化一次<自定义结构体>
static ONLINE_STATE: Lazy<OnlineState> = Lazy::new(|| OnlineState {
    user_activity_time: RwLock::new(HashMap::new()),
    user_activity_count: AtomicU32::new(0),
});

/// GET /heartbeat, 心脏检测
/// 
/// 可能有一些额外的服务器信息，如:
/// - 在线用户数
/// - 服务器时间
/// - 设备信息 (内存、CPU使用率等)
/// - 等
/// 
/// args:
/// - `cookie_jar` 用于获取或设置会话ID。
///   弊端: 如果客户端是非浏览器环境，而是自定义客户端，则需要该自定义客户端支持cookie
pub async fn get_heartbeat(
    cookie_jar: CookieJar,
) -> impl IntoResponse {
    // 获取会话id
    let (new_session_id, new_cookie_jar) =
        if let Some(cookie) = cookie_jar.get("session_id") { // 客户端带会话id
            let cookie_value = cookie.value().to_string();
            let user_activity = ONLINE_STATE.user_activity_time.read().await;

            if user_activity.contains_key(&cookie_value) { // 服务器有此id，沿用
                tracing::debug!("GET /heartbeat, cookies get {}", cookie_value);
                (cookie_value, cookie_jar)
            }
            else { // 服务端无此id，需重新分配
                let new_id = Uuid::new_v4().to_string();
                let cookie = Cookie::new("session_id", new_id.clone());
                tracing::warn!("GET /heartbeat, cookies reCreate {}", new_id);
                (new_id, cookie_jar.add(cookie.clone()))
            }
        }
        else { // 客户端不带会话id
            let new_id = Uuid::new_v4().to_string();
            let cookie = Cookie::new("session_id", new_id.clone());
            tracing::debug!("GET /heartbeat, Cookies: create {}", new_id);
            (new_id, cookie_jar.add(cookie.clone()))
        };

    // 更新用户活跃时间
    {
        let mut user_activity_time = ONLINE_STATE.user_activity_time.write().await;
        // insert会返回被替换值，若None则表示之前没有这个键，即这是个新用户
        let old_value = user_activity_time.insert(new_session_id.clone(), Instant::now());
        if old_value.is_none() {
            ONLINE_STATE.user_activity_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    let resp = json!({
        "status": "alive",
        "timestamp": chrono::Local::now().to_rfc3339(), // 本地时间
            // chrono::Utc::now().to_rfc3339(), // 零区
            // chrono::FixedOffset::east_opt(8 * 3600).unwrap(), // 东八区
        "online_user_count": ONLINE_STATE.user_activity_count.load(Ordering::Relaxed),
    });

    (new_cookie_jar, (StatusCode::OK, Json(resp)))
}

/// TODO 后台任务，定时清理不活跃用户
/// 
/// 检测到成出时间的用户，删除之，并使活跃用户数-1


/// 后台任务，定时清理不活跃用户
/// 
/// 检测到成出时间的用户，删除之，并使活跃用户数-1
/// 
/// args
/// - timeout_time 超时时间 (默认10)
/// - interval_time 检测频率 (略，默认5)
pub fn start_cleanup_task(timeout: Option<u64>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            
            // 移除超过30秒不活跃的用户
            let mut user_activity_time = ONLINE_STATE.user_activity_time.write().await;
            let before_count = user_activity_time.len();
            let now = Instant::now();
            user_activity_time.retain(|_, &mut last_active| now.duration_since(last_active) < Duration::from_secs(timeout.unwrap_or(10)));
            let after_count = user_activity_time.len();
            
            // 如果有变化则更新计数器
            if before_count != after_count {
                ONLINE_STATE.user_activity_count .store(after_count as u32, Ordering::Relaxed);
            }
        }
    });
}
