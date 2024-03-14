
# actix-webのミドルウェアの参考情報

## ミドルウェアの処理順序

`actix-web`のミドルウェアは、リクエストに対してミドルウェアを登録した順番と逆に、レスポンスに対して登録した順番に処理される。

```rust
#[get("/")]
async fn service(a: ExtractorA, b: ExtractorB) -> impl Responder { "Hello, World!" }

let app = App::new()
    .wrap(MiddlewareA)
    .wrap(MiddlewareB)
    .wrap(MiddlewareC)
    .service(service);
```

```text
                  Request
                     ⭣
╭────────────────────┼────╮
│ MiddlewareC        │    │
│ ╭──────────────────┼───╮│
│ │ MiddlewareB      │   ││
│ │ ╭────────────────┼──╮││
│ │ │ MiddlewareA    │  │││
│ │ │ ╭──────────────┼─╮│││
│ │ │ │ ExtractorA   │ ││││
│ │ │ ├┈┈┈┈┈┈┈┈┈┈┈┈┈┈┼┈┤│││
│ │ │ │ ExtractorB   │ ││││
│ │ │ ├┈┈┈┈┈┈┈┈┈┈┈┈┈┈┼┈┤│││
│ │ │ │ service      │ ││││
│ │ │ ╰──────────────┼─╯│││
│ │ ╰────────────────┼──╯││
│ ╰──────────────────┼───╯│
╰────────────────────┼────╯
                     ⭣
                  Response
```

## `actix-web::Either`について

```rust
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
```

`actix-web`の`Either`は、2つのエクストラクターまたはレスポンダー型を1つの型に結合する。

## エクストラクター

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

## レスポンダー

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

## [Returning an unauthorized response in actix-web middleware in Rust - stack overflow](https://stackoverflow.com/questions/68944823/returning-an-unauthorized-response-in-actix-web-middleware-in-rust)

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

## [actix_web::body::EitherBody](https://docs.rs/actix-web/latest/actix_web/body/enum.EitherBody.html)

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
