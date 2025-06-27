# README.dev

这个是给自己/开发者看的

记一些常用命令，避免忘记

## 编译

更多编译/环境相关见工作流文件

```bash
cargo run # dev

# or
cargo build
./target/debug/rust-http-demo.exe

# or
cargo build --release
./target/release/rust-http-demo.exe

# (option)
cargo check # 检查代码的正确性，而不编译。从而节省大量编译时间
```

## Project template

```bash
cargo new folderName
# - folderName/
#   - src/
#     - main.rs
#   - Cargo.toml

# b1. 命令版
# cargo add axum
# cargo add tokio@1.0 --features full

# b2. 直接复制黏贴以下内容到toml [dependencies]中
axum = "0.8.4" # 异步IO Web框架
tokio = { version = "1.0", features = ["full"] } # 异步IO
tracing = "0.1" # 日志追踪
tracing-subscriber = { version = "0.3", features = ["env-filter"] } # 日志追踪
# 并执行命令:
cargo build
```

## docs

```bash
cargo doc
# cargo doc --no-deps # 或不要生成第三方依赖的文档
# cargo doc --open # 或生成后直接打开
```
