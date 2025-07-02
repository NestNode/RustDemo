use std::collections::HashMap;
use once_cell::sync::Lazy;

type TaskFn = fn() -> ();

pub static NODE_LIST: Lazy<HashMap<String, TaskFn>> = Lazy::new(|| {
    let mut map: HashMap<String, TaskFn> = HashMap::new();
    map.insert("log".to_string(), log);
    map.insert("status".to_string(), get_status);
    map
});

fn log() {
    tracing::info!("log test");
}

fn get_status() {
    tracing::info!("status: running");
}
