# actix-webを利用したAPIサーバーのエラー処理とロギング

- [actix-webを利用したAPIサーバーのエラー処理とロギング](#actix-webを利用したapiサーバーのエラー処理とロギング)
  - [エラー処理](#エラー処理)
    - [エラー処理方針](#エラー処理方針)
    - [actix-webのエラー・ハンドラ・ミドルウェア](#actix-webのエラーハンドラミドルウェア)
    - [エラー処理の戦略](#エラー処理の戦略)
      - [カスタム・クライアント・エラー・ハンドラ](#カスタムクライアントエラーハンドラ)
      - [独自のエラー処理戦略](#独自のエラー処理戦略)
    - [actix-webのミドルウェアの参考](#actix-webのミドルウェアの参考)
      - [`actix-web::Either`について](#actix-webeitherについて)
      - [エクストラクター](#エクストラクター)
      - [レスポンダー](#レスポンダー)
      - [Returning an unauthorized response in actix-web middleware in Rust - stack overflow](#returning-an-unauthorized-response-in-actix-web-middleware-in-rust---stack-overflow)
      - [actix\_web::body::EitherBody](#actix_webbodyeitherbody)

## エラー処理

### エラー処理方針

- クライアント側のエラー処理を考慮して、エラー・レスポンスはすべてJSONで返す。
- エクストラクタがデシリアライズに失敗した場合など、`actix-web`がエラー処理したときも、レスポンスをJSONで返す。

### actix-webのエラー・ハンドラ・ミドルウェア

`actix-web`は、独自のエラー・ハンドラを登録する[ErrorHandlers](https://docs.rs/actix-web/latest/actix_web/middleware/struct.ErrorHandlers.html)ミドルウェアがある。
その`ErrorHandlers`ミドルウェアには、デフォルトのエラー・ハンドラや、ステータス・コードによって呼び出すエラー・ハンドラを登録できる。

```rust
fn add_error_header<B>(mut res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut().headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("Error"),
    );

    // ボディは変更されない、"左"の枠にマップする。
    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}

fn handle_bad_request<B>(mut res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut().headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("Bad Request Error"),
    );

    // ボディは変更されない、"左"の枠にマップする。
    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}

// `Bad Request`エラーは`handle_bad_request()`を叩く一方で、それ以外のすべてのエラーは
// `add_error_header()`を叩く。どのメソッドが呼び出されるかの順番は意味がない。
let app = App::new()
    .wrap(
        ErrorHandlers::new()
            .default_handler(add_error_header)
            .handler(StatusCode::BAD_REQUEST, handle_bad_request)
    )
    .service(web::resource("/").route(web::get().to(HttpResponse::InternalServerError)));
```

```sh
# ヘルス・チェック・エンドポイント
$ curl --include http://localhost:8080/
HTTP/1.1 200 OK
content-length: 9
date: Wed, 13 Mar 2024 04:27:15 GMT

It works!%

# 定義されていないエンドポイントへのリクエスト
$ curl --include http://localhost:8080/foo
HTTP/1.1 404 Not Found
content-length: 57
date: Wed, 13 Mar 2024 04:27:28 GMT

{"statusCode":404,"errorCode":null,"message":"Not Found"}%

# ログイン・エンドポイント
$ curl --include -X POST -H "Content-Type: application/json" -d '{"userName":"kuro","password":"test"}' http://localhost:8080/login
HTTP/1.1 200 OK
content-length: 37
content-type: application/json
date: Wed, 13 Mar 2024 04:29:15 GMT

{"message":"Authorization succeeded"}%

# ボディを指定せずにログイン・エンドポイントへのリクエスト
curl --include -X POST http://localhost:8080/login
HTTP/1.1 400 Bad Request
content-length: 59
content-type: text/plain; charset=utf-8
date: Wed, 13 Mar 2024 04:30:17 GMT

{"statusCode":400,"errorCode":null,"message":"Bad Request"}%

# 誤ったリクエスト・ボディを指定したログイン・エンドポイントへのリクエスト
$ curl --include -X POST -H "Content-Type: application/json" -d '{"user":"kuro","pass":"test"}' http://localhost:8080/login
HTTP/1.1 400 Bad Request
content-length: 59
content-type: text/plain; charset=utf-8
date: Wed, 13 Mar 2024 04:31:05 GMT

{"statusCode":400,"errorCode":null,"message":"Bad Request"}%
```

### エラー処理の戦略

#### カスタム・クライアント・エラー・ハンドラ

通常、`actix-web`は、リクエスト・ハンドラが呼び出されるまでに、URLディスパッチャやエクストラクターなどが、HTTPのクライアント・エラー(400 - 499)を処理する。
よって、クライアント・エラーに対して、独自のエラー・ハンドラを登録して、JSONでエラー・レスポンスを返す。

```rust
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
```

#### 独自のエラー処理戦略

`401 Unauthorized`や`403 Forbidden`のようなエラーは、認証に関わるエラーであるため、独自のミドルウェアやハンドラで処理した結果として返す。
また、HTTPのサーバー・エラー(500 - 599)は、リクエスト・ハンドラやユース・ケースのロジックで処理した結果として返す。

TODO: 独自のエラー処理戦略を検討する

### actix-webのミドルウェアの参考

#### `actix-web::Either`について

```rust
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
```

`actix-web`の`Either`は、2つのエクストラクターまたはレスポンダー型を1つの型に結合する。

#### エクストラクター

プライマリとフォールバックの、2つのエクストラクタを試行するメカニズムを提供する。
例えば、フォームがJSONまたはURLエンコードされている可能性がある、「多態性のある（polymorphic）ペイロード」に対して役にたつ。

このエクストラクターは、その実装の一部として、必然的にリクエストのペイロード全体をバッファすることに注意することが必要である。
しかし、このエクストラクターは、`PayloadConfig`の最大サイズ制限を尊重する。

```rust
use actix_web::{post, web, Either};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    name: String,
}

/// JSONまたはフォームURLエンコードでフォームを受け取るハンドラ
#[post("/")]
async fn index(form: Either<web::Json<Info>, web::Form<Into>>) -> String {
    let name: String = match form {
        Either::Left(json) => json.name.to_owned(),
        Either::Right(form) => form.name.to_owned(),
    };

    format!("Welcome {}!", name)
}
```

#### レスポンダー

複数の分岐がある具象レスポンス型を使用することが望ましい場合がある。
両方の型が`Responder`を実装する限り、`Either`型をハンドラの戻り値の型として使用できるようになる。

```rust
use actix_web::{get, Either, Error, HttpResponse};

#[get("/")]
async fn index() -> Either<&'static str, Result<HttpResponse, Error>> {
    if 1 == 2 {
        // 左側のバリアントで応答する。
        Either::Left("Bad data")
    } else {
        // 右側のバリアントで応答する。
        Either::Right(
            Ok(HttpResponse::Ok()
                .context_type(mime::TEXT_HTML)  // mimeクレート
                .body("<p>Hello!</p>"))
        )
    }
}
```

#### [Returning an unauthorized response in actix-web middleware in Rust - stack overflow](https://stackoverflow.com/questions/68944823/returning-an-unauthorized-response-in-actix-web-middleware-in-rust)

actix 4.0で、必要に応じて`Left`または`Right`を結果として使用するために、`Either`と呼ばれる型を導入されている。
例えば、`Left`は他のミドルウェアのレスポンスの型で、`Right`はあなたのミドルウェアの結果になり得る。
次のコードにおいて、JWTトークンが不正な場合に`UNAUTHORIZED`を返し、そうでない場合は他のミドルウェアの結果を返さなくてはならない。

```rust
use pin_project::pin_project;
use std::env;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_utils::future::{ok, Either, Ready};
use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::{Method, StatusCode};
use actix_web::{Error, HttpResponse};
use futures::{ready, Future};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

use crate::modules::user::models::Claims;

pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware { service })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Either<AuthenticationFuture<S, B>, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let mut authenticate_pass = false;
        if Method::OPTIONS == *req.method() {
            authenticate_pass = true;
        }

        let auth = req.headers().get("Authorization");
        if auth.is_some() {
            let split: Vec<&str> = auth.unwrap().to_str().unwrap().trim().split("Bearer").collect();
            let token = split[1].trim();
            let secret_key = env::var("SECRET_KEY").expect("SECRET_KEY in .env file is missing");
            let key = secret_key.as_bytes();
            if decode::<Claims>(token, &Decoding::Key::from_secret(key), &Validation::new(Algorithm::HS512)).is_ok() {
                authenticate_pass = true;
            }
        }

        if authenticate_pass {
            Either::left(AuthenticationFuture {
                fut: self.service.call(req),
                _phantom: PhantomData,
            })
        } else {
            let res = HttpResponse::with_body(StatusCode::UNAUTHORIZED, "Invalid JWT token");
            Either::right(ok(req.into_response(res).map_into_boxed_body().map_into_right_body()))
        }
    }
}

#[pin_project]
pub struct AuthenticationFuture<S, B>
where
    S: Service<ServiceRequest>,
{
    #[pin]
    fut: S::Future,
    _phantom: PhantomData<B>,
}

impl<S, B> Future for AuthenticationFuture<S, B>
where
    B: MessageBody,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Output = Result<ServiceResponse<EitherBody<B>>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = match ready!(self.project().fut.poll(cs)) {
            Ok(res) => res,
            Err(err) => return Poll::Ready(Err(err.into())),
        };

        Poll::Ready(Ok(res.map_into_left_body()))
    }
}
```

#### [actix_web::body::EitherBody](https://docs.rs/actix-web/latest/actix_web/body/enum.EitherBody.html)

```rust
pub enum EitherBody<L, R = BoxBody> {
    Left {
        body: L,
    },
    Right {
        body: R,
    },
}
```

ボディ型の特別な`Either`型である。

特にミドルウェアにおいて、条件付きで内部のサービスの未知／汎用的なボディ`B`型を返すか、新しいレスポンスで早期リターンすることが一般的である。
エラー・レスポンスが一般的であるため、この型の「右」バリアンとは`BoxBody`がデフォルトである。

例えば、ミドルウェアは、よく`Response = ServiceResponse<EitherBody<B>>`を使用する。
これは、内部サービスのレスポンス・ボディの型を`Left`バリアントにマップして、ミドルウェア独自のエラー・レスポンスを`BoxBody`がデフォルトの`Right`バリアントを使用する。
もちろん、代わりのレスポンスが既知の型であれば、代わりに`EitherBody<B, String>`を使用できない理由はない（を使用しても良い）。
