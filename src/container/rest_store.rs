use std::collections::HashMap;
use std::sync::{Arc, RwLock}; // 线程安全共享指针和读写锁
// use std::thread;

/// 一个线程安全的容器，封装了一个具有字符串键和泛型值的HashMap。
/// 
/// 特性：
/// - 使用RwLock确保线程安全操作
/// - 字符串类型的键，泛型类型的值
/// - 基本操作：get、put、delete、...
/// 
/// 为安全性，禁止直接编辑返回的元素。这样只需要保证容器是多线程安全的就行了
#[derive(Debug, Clone)]
pub struct Container<T> {
    data: Arc<RwLock<HashMap<String, T>>>,
}

impl<T> Container<T> {
    /// 创建对象
    fn new() -> Self {
        Container {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建对象
    pub fn new_arc() -> Arc<Container<T>> {
        Arc::new(Container::<T>::new())
    }

    // ---------------- 增删改查 ----------------

    /// 获取
    pub fn get_by_id(&self, key: &str) -> Option<T>
    where
        T: Clone,
    {
        let map = self.data.read().unwrap();
        map.get(key).cloned()
    }

    /// 获取 - 全部
    pub fn get_all(&self) -> HashMap<String, T>
    where
        T: Clone,
    {
        let map = self.data.read().unwrap();
        map.clone()
    }

    /// 获取 - 键是否存在
    pub fn _get_is(&self, key: &str) -> bool {
        let map = self.data.read().unwrap();
        map.contains_key(key)
    }

    // /// 增加 - 随机
    // 略，由上层实现

    /// 增加 - 覆盖
    pub fn put_by_id(&self, key: &str, value: T) -> Option<T> {
        let mut map = self.data.write().unwrap();
        map.insert(key.to_string(), value)
    }

    // /// 增加 - 新增
    // 略，由上层实现

    /// 删除
    pub fn delete_by_id(&self, key: &str) -> Option<T> {
        let mut map = self.data.write().unwrap();
        map.remove(key)
    }

    /// 删除 - 清空
    pub fn _delete_all(&self) {
        let mut map = self.data.write().unwrap();
        map.clear();
    }

    // ---------------- 其他 --------------------

    /// 获取当前元素数量
    pub fn _len(&self) -> usize {
        let map = self.data.read().unwrap();
        map.len()
    }

    /// 检查容器是否为空
    pub fn _is_empty(&self) -> bool {
        let map = self.data.read().unwrap();
        map.is_empty()
    }
}

// 实现Default trait，提供便利
impl<V> Default for Container<V>
where
    V: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/*
// 添加静态测试方法
impl Container<i32> {
    /// 静态测试方法，展示ThreadSafeContainer的基本用法
    pub fn test() {
        println!("=== 测试整数类型容器 ===");
        
        // 创建一个整数类型的容器
        let int_container = Container::<i32>::new();
        
        // 添加一些值
        int_container.put_by_id("one".to_string(), 1);
        int_container.put_by_id("two".to_string(), 2);
        int_container.put_by_id("three".to_string(), 3);
        
        println!("容器包含 {} 个项目", int_container.len());
        println!("键'one'对应的值: {:?}", int_container.get_by_id("one"));
        
        // 删除一个项目
        int_container.delete_by_id("two");
        println!("删除后，容器包含'two': {}", int_container.contains_key("two"));
    }
}

impl Container<String> {
    /// 静态测试方法，展示ThreadSafeContainer的字符串用法
    pub fn test() {
        println!("=== 测试字符串类型容器 ===");
        
        // 使用字符串值
        let string_container = Container::<String>::new();
        string_container.put_by_id("greeting".to_string(), "你好，世界！".to_string());
        println!("greeting: {:?}", string_container.get_by_id("greeting"));
        
        // 线程安全性演示
        println!("\n=== 测试多线程安全性 ===");
        let container = Container::<String>::new();
        container.put_by_id("shared".to_string(), "初始值".to_string());
        
        let container_ref = &container;
        
        let handles: Vec<_> = (0..5).map(|i| {
            let container = container_ref;
            thread::spawn(move || {
                // 读取操作是线程安全的
                println!("线程 {}: 当前值是 {:?}", i, container.get_by_id("shared"));
                
                // 写入操作也是线程安全的
                container.put_by_id(format!("thread_{}", i), format!("来自线程 {}", i));
                
                if i == 3 {
                    // 一个线程更新共享值
                    container.put_by_id("shared".to_string(), format!("被线程 {} 更新", i));
                }
            })
        }).collect();
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
        
        // 打印最终状态
        println!("'shared'的最终值: {:?}", container.get_by_id("shared"));
        // println!("所有键: {:?}", container.keys());
    }
}*/
