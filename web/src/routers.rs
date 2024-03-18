use std::borrow::Cow;

use actix_web::body::BoxBody;
use actix_web::http::{header, StatusCode};
use actix_web::{web, HttpResponse, HttpResponseBuilder, Responder, ResponseError};
use macros::ResponseErrorImpl;
use uuid::Uuid;

use crate::use_cases::{self, RegisterUserError, RegistrationUser};

/// Content-Type
pub const CONTENT_TYPE_JSON: &str = "application/json";

/// エラー・レスポンス・ボディ
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseBody<'a> {
    /// HTTPステータス・コード
    status_code: u16,

    /// アプリ独自のエラー・コード
    ///
    /// `actix-web`がエラー処理した場合は`None`である。
    error_code: Option<u32>,

    /// エラー・メッセージ
    message: Cow<'a, str>,
}

impl<'a> ErrorResponseBody<'a> {
    pub fn new<T>(status_code: u16, error_code: Option<u32>, message: T) -> Self
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

/// ヘルス・チェック
#[tracing::instrument(
    name = "health check",
    fields(request_id = %Uuid::new_v4())
)]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("It works!")
}

/// ログイン・リクエスト・ボディ
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestBody {
    /// ユーザー名
    user_name: String,
    /// パスワード
    password: String,
}

/// ログイン・レスポンス・ボディ
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseBody<'a> {
    message: Cow<'a, str>,
}

/// ログイン
#[tracing::instrument(
    name = "login",
    skip(body),     // パスワードをログに出力しないようにスキップ
    fields(
        request_id = %Uuid::new_v4(),
        user_name = %body.user_name,
    )
)]
pub async fn login(body: web::Json<LoginRequestBody>) -> impl Responder {
    let _user_name = &body.user_name;
    let _password = &body.password;

    HttpResponse::Ok().json(LoginResponseBody {
        message: "Authorization succeeded".into(),
    })
}

/// ユーザー登録リクエスト・ボディ
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationUserRequestBody {
    /// ユーザー名
    user_name: String,
    /// パスワード
    password: String,
}

impl ResponseError for RegisterUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::WeakPassword => StatusCode::BAD_REQUEST,
            Self::UserAlreadyExists(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let status_code = self.status_code();
        let error_code: Option<u32> = match self {
            Self::Unexpected(_) => Some(1),
            Self::Repository(_) => Some(2),
            Self::WeakPassword => Some(10000),
            Self::UserAlreadyExists(_) => Some(10001),
        };
        let message = format!("{}", self);
        let body = ErrorResponseBody::new(status_code.as_u16(), error_code, message);

        HttpResponseBuilder::new(status_code)
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .json(body)
    }
}

/// ユーザー登録
#[tracing::instrument(
    name = "register user",
    skip(body),     // パスワードをログに出力しないようにスキップ
    fields(
        request_id = %Uuid::new_v4(),
        user_name = %body.user_name,
    )
)]
pub async fn register_user(
    body: web::Json<RegistrationUserRequestBody>,
) -> Result<HttpResponse, RegisterUserError> {
    let user = RegistrationUser {
        user_name: body.user_name.clone(),
        password: body.password.clone(),
    };

    use_cases::register_user(user).await?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, thiserror::Error, ResponseErrorImpl)]
pub enum TestError {
    /// 予期しないエラー
    #[error("Unexpected error: {0}")]
    #[response_error(status_code = 500, error_code = 1)]
    Unexpected(anyhow::Error),

    /// パスワードが弱い
    #[error("Password is weak")]
    #[response_error(status_code = 400, error_code = 1000)]
    WeakPassword,
}
