use actix_web::middleware::ErrorHandlers;
use actix_web::{web, App, HttpServer};
use tracing_actix_web::TracingLogger;

use ::web::routers::{default_error_handler, health_check, login, register_user};
use ::web::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // サブスクライバを初期化
    let subscriber = get_subscriber("error_and_logging".into(), "info".into());
    init_subscriber(subscriber);

    tracing::info!("start program");

    HttpServer::new(|| {
        App::new()
            .wrap(ErrorHandlers::new().default_handler(default_error_handler))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(health_check))
            .route("/login", web::post().to(login))
            .route("/users", web::post().to(register_user))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
