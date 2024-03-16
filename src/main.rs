use actix_web::dev::ServiceResponse;
use actix_web::http::header;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{web, App, HttpServer};
use actix_web_handle_error_and_logging::telemetry::{get_subscriber, init_subscriber};
use tracing_actix_web::TracingLogger;

use actix_web_handle_error_and_logging::routers::{
    health_check, login, register_user, ErrorResponseBody, CONTENT_TYPE_JSON,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // サブスクライバを初期化
    let subscriber = get_subscriber("actix_web_handle_error_and_logging".into(), "info".into());
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

/// カスタム・デフォルト・エラー・ハンドラ
fn default_error_handler<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
    // コンテンツ・タイプがapplication/jsonの場合はそのまま返す
    let content_type = res.headers().get(header::CONTENT_TYPE);
    if content_type.is_some() && content_type.unwrap() == CONTENT_TYPE_JSON {
        return Ok(ErrorHandlerResponse::Response(res.map_into_left_body()));
    }

    // エラー・レスポンス・ボディを生成
    let status_code = res.status().as_u16();
    let message = res
        .status()
        .canonical_reason()
        .unwrap_or("Unexpected error raised");
    let body = ErrorResponseBody::new(status_code, None, message);
    let body = serde_json::to_string(&body).unwrap();
    let (req, res) = res.into_parts();
    let mut res = res.set_body(body);
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(CONTENT_TYPE_JSON),
    );
    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}
