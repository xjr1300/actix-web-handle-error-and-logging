use std::borrow::Cow;
use std::str::FromStr as _;

use actix_web::body::BoxBody;
use actix_web::dev::ServiceResponse;
use actix_web::http::header::{self, HeaderMap};
use actix_web::http::StatusCode;
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::{web, HttpResponse, HttpResponseBuilder, Responder};
use mime::Mime;
use uuid::Uuid;

use crate::use_cases::{self, RegisterUserError, RegistrationUser};

/// エラー・レスポンス・ボディ
///
/// アプリケーションから返されるエラー・レスポンスのボディを表現する。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseBody<'a> {
    /// アプリ独自のエラー・コード
    ///
    /// `actix-web`がエラー処理した場合は`None`である。
    error_code: Option<u32>,

    /// エラー・メッセージ
    message: Cow<'a, str>,
}

impl<'a> ErrorResponseBody<'a> {
    pub fn new<T>(error_code: Option<u32>, message: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Self {
            error_code,
            message: message.into(),
        }
    }
}

/// HTTPヘッダからContent-Typeを取得する。
///
/// # 引数
///
/// * `headers` - HTTPヘッダ
///
/// # 戻り値
///
/// * `Mime`
/// * Content-Typeが設定されていない場合は`None`
fn retrieve_content_type(headers: &HeaderMap) -> Option<Mime> {
    let content_type = headers.get(header::CONTENT_TYPE)?;
    let content_type = content_type.to_str().ok()?;
    match Mime::from_str(content_type) {
        Ok(mime) => Some(mime),
        Err(_) => None,
    }
}

/// カスタム・デフォルト・エラー・ハンドラ
pub fn default_error_handler<B>(
    res: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    // コンテンツ・タイプがapplication/jsonの場合はそのまま返す
    let content_type = retrieve_content_type(res.headers());
    if content_type.is_some() && content_type.unwrap() == mime::APPLICATION_JSON {
        return Ok(ErrorHandlerResponse::Response(res.map_into_left_body()));
    }

    // レスポンス・ボディを生成
    let message = res
        .status()
        .canonical_reason()
        .unwrap_or("Unexpected error raised");
    let body = ErrorResponseBody::new(None, message);
    let body = serde_json::to_string(&body).unwrap();
    let (req, res) = res.into_parts();
    let mut res = res.set_body(body);
    // レスポンスのヘッダを`application/json`に設定
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(mime::APPLICATION_JSON.as_ref()).unwrap(),
    );
    // レスポンスを構築
    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
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
pub struct RegisterUserRequestBody {
    /// ユーザー名
    user_name: String,
    /// パスワード
    password: String,
}

/// ユーザー登録
#[tracing::instrument(
    name = "register user",
    skip(body),     // パスワードをログに出力しないように、リクエスト・ボディ全体をスキップ
    fields(
        request_id = %Uuid::new_v4(),   // リクエストIDを設定して、リクエストに対する一連の処理をログで追跡
        user_name = %body.user_name,    // リクエスト・ボディのユーザー名をログに記録
    )
)]
pub async fn register_user(body: web::Json<RegisterUserRequestBody>) -> HttpResponse {
    let user = RegistrationUser {
        user_name: body.user_name.clone(),
        password: body.password.clone(),
    };

    match use_cases::register_user(user).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => err.into(),
    }
}

/// `RegisterUserError`を`HttpResponse`に変換
impl From<RegisterUserError> for HttpResponse<BoxBody> {
    fn from(value: RegisterUserError) -> Self {
        let status_code = match value {
            RegisterUserError::Unexpected(..) | RegisterUserError::Repository(..) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            RegisterUserError::WeakPassword | RegisterUserError::UserAlreadyExists(..) => {
                StatusCode::BAD_REQUEST
            }
        };
        let body = ErrorResponseBody::new(Some(value.error_code()), value.to_string());

        HttpResponseBuilder::new(status_code)
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .json(body)
    }
}
