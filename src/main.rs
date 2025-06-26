//! 主程序入口模块
//! 
//! 负责服务器配置和启动

use axum::{
    http::{HeaderName, HeaderValue, Method},
    routing::get,
    Router
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt}; // 日志订阅系统
mod api;

/// 主异步函数，使用tokio运行时
#[tokio::main]
async fn main() {
    api::test::test_fn();

    // 初始化日志追踪
    tracing_subscriber::registry()
        .with( // 过滤规则: 默认显示debug级别
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer()) // 输出格式
        .init(); // 初始化

    // axum
    let cors = CorsLayer::new()
        .allow_origin(
            Any,
            // #[cfg(debug_assertions)]
            // Any,
            
            // #[cfg(not(debug_assertions))]
            // [
            //     "http://localhost".parse::<HeaderValue>().unwrap(),
            //     "http://localhost:3060".parse::<HeaderValue>().unwrap(),
            // ],
        ) // Any 允许任意来源，开发阶段可用，生产建议指定域名
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
            HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(
            false,
            // 允许凭证 (cookies等)。但若开了，限制不再允许用 `allow_origin(Any)`，因为这会带来严重的安全风险
            // #[cfg(debug_assertions)]
            // false,

            // #[cfg(not(debug_assertions))]
            // true,
        )
        ;
    let app = Router::new()
        .route("/", get(api::test::root))
        .merge(api::heartbeat::factory_utils_router())
        .merge(api::todos::factory_todos_router().await)
        .merge(api::rest::factory_rest_router().await)
        .merge(api::node::factory_node_router().await)
        .layer(cors);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:24042") // 绑定TCP监听端口
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap(); // 启动HTTP服务器
}
