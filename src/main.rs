//! 主程序入口模块
//! 
//! 负责服务器配置和启动

use axum::{response::Html, routing::get, Router};

#[tokio::main]
async fn main() {
    // 使用路由构建我们的应用程序
    let app = Router::new().route("/", get(handler));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:24042")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
