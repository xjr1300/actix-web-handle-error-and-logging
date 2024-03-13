use actix_web::{web, App, HttpResponse, HttpServer, Responder};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/", web::get().to(health_check)))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

/// ヘルス・チェック
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("It works!")
}
