use std::borrow::Cow;

use actix_web::{web, HttpResponse, Responder};

/// ヘルス・チェック
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("It works!")
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestBody {
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

/// ログイン
pub async fn login(body: web::Json<LoginRequestBody>) -> impl Responder {
    let _user_name = &body.user_name;
    let _password = &body.password;

    HttpResponse::Ok().json(LoginResponseBody {
        message: "Authorization succeeded".into(),
    })
}
