//! 用于心跳检测的API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
    // extract::ConnectInfo,
    Extension,
};
// use axum::extract::CookieJar;
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use serde_json::json;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::{
    time::{Duration, Instant},
    collections::HashMap,
    // net::SocketAddr,
    sync::{
        atomic::{AtomicU32, Ordering},
    },
};

/// 工具路由
/// 
/// 包括心跳检测和常用工具等
pub fn factory_utils_router() -> Router {
    // 启动清理任务
    // start_cleanup_task();

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
    // 新会话id
    let new_cookie_jar = if let Some(cookie) = cookie_jar.get("session_id") {
        tracing::debug!("GET /heartbeat, cookies get {}", cookie.value().to_string());

        // TODO ONLINE_STATE中如果找到id，则更新其时间，否则重新分配uuid
        
        cookie_jar
    } else {
        let new_id = Uuid::new_v4().to_string();
        tracing::debug!("GET /heartbeat, Cookies: create {}", new_id);
        let cookie = Cookie::new("session_id", new_id.clone());
        
        // TODO ONLINE_STATE插入新用户，并增加用户数
        
        cookie_jar.add(cookie)
    };

    // 更新用户活跃时间
    // {
    //     let mut user_activity = ONLINE_STATE.user_activity_time.write().await;
        
    //     // 如果是新用户，增加在线计数
    //     if user_activity.insert(session_id.clone(), Instant::now()).is_none() {
    //         ONLINE_STATE.user_activity_count.fetch_add(1, Ordering::Relaxed);
    //     }
    // }

    let resp = json!({
        "status": "alive",
        "timestamp": chrono::Local::now().to_rfc3339(), // 本地时间
            // chrono::Utc::now().to_rfc3339(), // 零区
            // chrono::FixedOffset::east_opt(8 * 3600).unwrap(), // 东八区
        "online_user_count": ONLINE_STATE.user_activity_count.load(Ordering::Relaxed),
    });

    (new_cookie_jar, (StatusCode::OK, Json(resp)))
}

/*/// TODO 后台任务，定时清理不活跃用户
/// 
/// 检测到成出时间的用户，删除之，并使活跃用户数-1


/// 后台任务，定时清理不活跃用户
pub fn start_cleanup_task() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            
            // 移除超过30秒不活跃的用户
            let mut user_activity = ONLINE_STATE.user_activity_time.write().await;
            let before_count = user_activity.len();
            let now = Instant::now();
            user_activity.retain(|_, &mut last_active| now.duration_since(last_active) < Duration::from_secs(30));
            let after_count = user_activity.len();
            
            // 如果有变化则更新计数器
            if before_count != after_count {
                ONLINE_STATE.user_activity_count .store(after_count as u32, Ordering::Relaxed);
            }
        }
    });
}*/
