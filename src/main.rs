use std::borrow::Cow;

use actix_web::dev::ServiceResponse;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(ErrorHandlers::new().default_handler_client(client_error_handler))
            .route("/", web::get().to(health_check))
            .route("/login", web::post().to(login))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

/// ヘルス・チェック
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("It works!")
}

/// ログイン
async fn login(body: web::Json<LoginRequestBody>) -> impl Responder {
    let _user_name = &body.user_name;
    let _password = &body.password;

    HttpResponse::Ok().json(LoginResponseBody {
        message: "Authorization succeeded".into(),
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequestBody {
    /// ユーザー名
    user_name: String,
    /// パスワード
    password: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseBody<'a> {
    message: Cow<'a, str>,
}

/// エラー・メッセージ・ボディ
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponseBody<'a> {
    /// HTTPステータス・コード
    status_code: u16,
    /// アプリ独自のエラー・コード
    ///
    /// `actix-web`がエラー処理した場合は`None`である。
    error_code: Option<u16>,
    /// エラー・メッセージ
    message: Cow<'a, str>,
}

impl<'a> ErrorResponseBody<'a> {
    fn new<T>(status_code: u16, error_code: Option<u16>, message: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            status_code,
            error_code,
            message: message.into(),
        }
    }
}

/// カスタム・クライアント・エラー・ハンドラ
fn client_error_handler<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
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
