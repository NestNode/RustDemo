//! 主程序入口模块
//! 
//! 负责服务器配置和启动

use axum::{
    http::{HeaderName, Method},
    routing::get,
    Router
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{ // 日志订阅系统
    layer::SubscriberExt,
    util::SubscriberInitExt
};

mod container;
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
        .with(tracing_subscriber::fmt::layer()) // 默认输出格式
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
        .merge(api::rest_todos::factory_todos_router().await)
        .merge(api::rest_store::factory_rest_router().await)
        .merge(api::rest_node::factory_node_router().await)
        .layer(cors);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:24042") // 绑定TCP监听端口
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap(); // 启动HTTP服务器
}

// /// 自定义日志的格式化器
// /// 
// /// 调换了打印内容和打印来源，以便对打印内容进行对齐
// /// 缺点：损失了着色
// ///
// /// 使用:
// /// .with( // 自定义输出格式
// ///     tracing_subscriber::fmt::layer()
// ///         .event_format(CustomEventFormatter)
// /// )
// ///
// /// 如果只需要隐藏来源，可以直接在 `layout()` 后加上 `.with_target(false)`
// struct CustomEventFormatter;
// impl<S, N> FormatEvent<S, N> for CustomEventFormatter
// where
//     S: tracing::Subscriber + for<'a> LookupSpan<'a>,
//     N: for<'a> FormatFields<'a> + 'static,
// {
//     fn format_event(
//         &self,
//         ctx: &FmtContext<'_, S, N>,
//         mut writer: Writer<'_>,
//         event: &tracing::Event<'_>,
//     ) -> fmt::Result {
//         // 获取当前时间
//         let now = chrono::Utc::now();
        
//         // 写入时间戳
//         write!(writer, "{} ", now.format("%Y-%m-%dT%H:%M:%S%.6fZ"))?;
        
//         // 写入日志级别
//         write!(writer, "{:5} ", event.metadata().level())?;
        
//         // 写入消息内容（这是你的"打印内容"）
//         ctx.field_format().format_fields(writer.by_ref(), event)?;
        
//         // 写入目标（这是你的"打印来源"，现在放在后面）
//         write!(writer, " {}", event.metadata().target())?;
        
//         writeln!(writer)
//     }
// }
