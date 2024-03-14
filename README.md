# actix-webを利用したAPIサーバーのエラー処理とロギング

- [actix-webを利用したAPIサーバーのエラー処理とロギング](#actix-webを利用したapiサーバーのエラー処理とロギング)
  - [エラー処理](#エラー処理)
    - [要求仕様](#要求仕様)
    - [エラー・レスポンス・ボディ](#エラーレスポンスボディ)
    - [エラー・レスポンスを加工する方法](#エラーレスポンスを加工する方法)
    - [エラー・レスポンスを生成する方法](#エラーレスポンスを生成する方法)
    - [エラー処理の実装概要](#エラー処理の実装概要)

## エラー処理

### 要求仕様

- クライアント側のエラー処理を考慮して、エラー・レスポンスはすべてJSONで返す。
- エクストラクタがデシリアライズに失敗した場合など、`actix-web`がエラー処理したときも、レスポンスをJSONで返す。

### エラー・レスポンス・ボディ

本サンプルでは、エラー・レスポンスのボディを次の通り定義する。

```rust
/// エラー・レスポンス・ボディ
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseBody<'a> {
    /// HTTPステータス・コード
    status_code: u16,

    /// アプリ独自のエラー・コード
    ///
    /// `actix-web`がエラー処理した場合は`None`である。
    error_code: Option<u16>,

    /// エラー・メッセージ
    message: Cow<'a, str>,
}
```

`error_code`フィールドは、発生したエラーを識別するアプリ独自のエラー・コードを示す。
`actix-web`がエラー処理した場合、`error_code`フィールドは`None`である。
なお、エラー・レスポンス・ボディ構造体をJSONにシリアライズするために、`serde`クレートを使用する。

### エラー・レスポンスを加工する方法

`actix-web`は、エラー・レスポンスを加工する手段を[ErrorHandlers](https://docs.rs/actix-web/latest/actix_web/middleware/struct.ErrorHandlers.html)で提供している。
本サンプルでは、[ErrorHandlers::default_handler()](https://docs.rs/actix-web/latest/actix_web/middleware/struct.ErrorHandlers.html#method.default_handler)でエラー・レスポンスを加工する。
`ErrorHandler`は、次のエラー・レスポンスを処理する次のメソッドを提供している。
なお、次のエラー・レスポンスを処理するメソッドのことを、**デフォルト・エラー・ハンドラ**と呼ぶ。

- `default_handler_client()`: クライアント・エラー(ステータス・コード400-499)を処理する。
- `default_handler_server()`: サーバー・エラー(ステータス・コード500-599)を処理する。
- `default_handler()`: 上記2つのエラーすべてを処理する。

`actix-web`は、エクストラクタなどでエラー処理した場合、そのエラー・レスポンスにはボディがないため、ヘッダにコンテンツ・タイプが設定されていない。

```sh
$ curl --include http://localhost:8080/foo
HTTP/1.1 404 Not Found
content-length: 0
date: Thu, 14 Mar 2024 08:06:11 GMT
```

よって、デフォルト・エラー・ハンドラでは、これらのエラー・レスポンスを上記エラー・レスポンス・ボディ構造体の内容をJSONにしたエラー・レスポンスを返す。
また、もしデフォルト・エラー・ハンドラがコンテンツ・タイプが`application/json`であるエラー・レスポンスを受け取った場合、そのエラー・レスポンスを加工せずにそのまま返す。

### エラー・レスポンスを生成する方法

クリーン・アーキテクチャにおけるユース・ケース層では、ユース・ケースごとに独自のエラー型をユース・ケース層に定義する。
そして、その独自エラー型に、[ResponseError](https://docs.rs/actix-web/latest/actix_web/error/trait.ResponseError.html)トレイトをコントローラー層（ルーティングを定義した層）を実装することで、`actix-web`が独自エラー型からエラー・レスポンスを生成できるようにする。
なお、`ResponseError::error_response()`メソッドには、エラーの内容をエラー・レスポンス・ボディ構造体に変換する処理を実装する。

> それぞれの独自エラー型に`ResponseError`トレイトを実装することは煩雑なため、マクロを利用して実装する。
>
> ユース・ケース層でエラー処理したときのエラー・レスポンスには、デフォルト・エラー・ハンドラで加工されない。

### エラー処理の実装概要

```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            // デフォルト・エラー・ハンドラをミドルウェアとして登録
            .wrap(ErrorHandlers::new().default_handler(default_error_handler))
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
    // actix-webが処理したエラーのエラー・レスポンス・ボディをJSONに変更
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
```

```rust
/// ユース・ケース層
#[derive(Debug, thiserror::Error)]
pub enum RegisterUserError {
    /// 予期しないエラー
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),

    /// リポジトリ・エラー
    #[error(transparent)]
    Repository(#[from] anyhow::Error),

    /// パスワードが弱い
    #[error("Password is weak")]
    WeakPassword,

    /// ユーザー名が既に登録されている
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
}

/// コントローラー層（ルーティングを定義した層）
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
        let error_code: Option<u16> = match self {
            Self::Unexpected(_) => Some(1),
            Self::Repository(_) => Some(2),
            Self::WeakPassword => Some(10000),
            Self::UserAlreadyExists(_) => Some(10001),
        };
        let message = format!("{}", self);
        let body = ErrorResponseBody::new(status_code.as_u16(), error_code, message);
        let body = serde_json::to_string(&body).unwrap();

        HttpResponse::new(status_code).set_body(body.boxed())
    }
}
```