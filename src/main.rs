use actix_web::dev::ServiceResponse;
use actix_web::http::header;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{web, App, HttpServer};

use actix_web_handle_error::routers::{
    health_check, login, register_user, ErrorResponseBody, CONTENT_TYPE_JSON,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(ErrorHandlers::new().default_handler_client(client_error_handler))
            .route("/", web::get().to(health_check))
            .route("/login", web::post().to(login))
            .route("/users", web::post().to(register_user))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

/// カスタム・クライアント・エラー・ハンドラ
fn client_error_handler<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
    // Content-Typeがapplication/jsonの場合はそのまま返す
    let content_type = res.headers().get(header::CONTENT_TYPE);
    if content_type.is_some() && content_type.unwrap() == CONTENT_TYPE_JSON {
        return Ok(ErrorHandlerResponse::Response(res.map_into_left_body()));
    }

    // actix-webが処理したエラー・レスポンス・ボディをJSONに変更
    let status_code = res.status().as_u16();
    let error_code: Option<u16> = None;
    let message = res
        .status()
        .canonical_reason()
        .unwrap_or("Unexpected error raised");
    let body = ErrorResponseBody::new(status_code, error_code, message);
    let body = serde_json::to_string(&body).unwrap();

    let (req, res) = res.into_parts();
    let res = res.set_body(body);

    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}
