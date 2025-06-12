//! 主程序入口模块
//! 
//! 负责服务器配置和启动

use axum::{
    response::Html,
    routing::{get},
    Router
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt}; // 日志订阅系统
mod api;

// use crate::api::test;

// use crate::api::todos::todos_handler;
// 新增：引入todo模块
// mod api {
//     pub mod todo;
// }
// use api::todo::{todo_router, Db};

// use std::{collections::HashMap, sync::{Arc, RwLock}};


// 主异步函数，使用tokio运行时
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
    let app = Router::new() // 路由
        .route("/", get(root)) // root
        .merge(api::todos::factory_todos_router().await) // todos
        .merge(api::rest::factory_rest_router().await); // rest
    let listener = tokio::net::TcpListener::bind("127.0.0.1:24042") // 绑定TCP监听端口
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap(); // 启动HTTP服务器
}

// root
async fn root() -> Html<&'static str> {
    tracing::debug!("GET /");
    Html("<h1>Hello, World!</h1>")
}
