//! Axum 的 web api
//! 
//! 区分成多个模块，作为多个API组
//! 符合 RESTful 风格

pub mod test;
pub mod heartbeat;
pub mod rest_todos;
pub mod rest_store;
pub mod rest_node;
