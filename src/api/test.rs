//! 仅测试

use axum::{
    response::Html
};

/// GET / 测试是否服务器正常
pub async fn root() -> Html<&'static str> {
    tracing::debug!("GET /");
    Html("<h1>Hello, World!</h1>")
}

pub fn test_fn() {
    println!("This is a test function.");
}
