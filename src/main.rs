use actix_web::{get, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Rust HTTP Server!")
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Hello, world!");

    HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(health_check)
    })
    .bind(("0.0.0.0", 24042))?
    .run()
    .await
}
