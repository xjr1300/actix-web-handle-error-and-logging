# actix-webを利用したAPIサーバーのエラー処理とロギング

- [actix-webを利用したAPIサーバーのエラー処理とロギング](#actix-webを利用したapiサーバーのエラー処理とロギング)
  - [エラー処理](#エラー処理)
    - [要求仕様](#要求仕様)
    - [エラー・レスポンス・ボディ](#エラーレスポンスボディ)
    - [`actix-web`がエラー処理した場合にエラー・レスポンスを加工する方法](#actix-webがエラー処理した場合にエラーレスポンスを加工する方法)
    - [ユース・ケース層における実装](#ユースケース層における実装)
    - [コントローラー（ルーティング）層における実装](#コントローラールーティング層における実装)
  - [ロギング](#ロギング)
    - [要求仕様](#要求仕様-1)
    - [実装方針](#実装方針)
    - [実装概要](#実装概要)

## エラー処理

### 要求仕様

- クライアント側のエラー処理を考慮して、エラー・レスポンスはすべてJSONで返す。
- エクストラクタがデシリアライズに失敗した場合など、`actix-web`がエラー処理したときも、レスポンスをJSONで返す。

### エラー・レスポンス・ボディ

本サンプルでは、エラー・レスポンスのボディを次の通り定義する。

- エラー・レスポンス・ボディは、次のフィールドを持つ。
  - `error_code`: アプリ独自のエラー・コード
  - `message`: エラー・メッセージ
- `actix-web`がエラー処理した場合、`error_code`フィールドは`None`
- エラー・レスポンス・ボディを`serde`クレートを使用してJSONにシリアライズ
- JSONにシリアライズする際、フィールド名をキャメルケースに変換

```rust
/// エラー・レスポンス・ボディ
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseBody<'a> {
    /// アプリ独自のエラー・コード
    ///
    /// `actix-web`がエラー処理した場合は`None`とする。
    error_code: Option<u16>,

    /// エラー・メッセージ
    message: Cow<'a, str>,
}
```

### `actix-web`がエラー処理した場合にエラー・レスポンスを加工する方法

`actix-web`は、エラー・レスポンスを加工する手段を[ErrorHandlers](https://docs.rs/actix-web/latest/actix_web/middleware/struct.ErrorHandlers.html)で提供している。
本サンプルでは、[ErrorHandlers::default_handler()](https://docs.rs/actix-web/latest/actix_web/middleware/struct.ErrorHandlers.html#method.default_handler)でエラー・レスポンスを加工する。
なお、`ErrorHandler`は、次のエラー・レスポンスを処理する次のメソッドを提供している。
また、本サンプルでは、次のエラー・レスポンスを処理するメソッドのことを**デフォルト・エラー・ハンドラ**と呼ぶ。

- `default_handler_client()`: クライアント・エラー(ステータス・コード400-499)を処理する。
- `default_handler_server()`: サーバー・エラー(ステータス・コード500-599)を処理する。
- `default_handler()`: 上記2つのエラーすべてを処理する。

`actix-web`がエラー処理したときの、レスポンスを次に示す。

```sh
$ curl --include http://localhost:8080/foo
HTTP/1.1 404 Not Found
content-length: 0
date: Thu, 14 Mar 2024 08:06:11 GMT
```

`actix-web`は、エクストラクタなどでエラー処理した場合、レスポンス・ボディがなく、ヘッダにコンテンツ・タイプが設定されていない。
よって、デフォルト・エラー・ハンドラは、コンテンツ・タイプが`application/json`でないレスポンスを受け取った場合、エラー内容を上記`ErrorResponseBody`構造体に変換した値をJSONにシリアライズしてレスポンス・ボディに設定する。
なお、デフォルト・エラー・ハンドラが、コンテンツ・タイプが`application/json`であるレスポンスを受け取った場合、そのレスポンスを加工せずにそのまま返す。

### ユース・ケース層における実装

クリーン・アーキテクチャにおけるユース・ケース層で、それぞれのユース・ケースごとに独自のエラー型を列挙型でユース・ケース層に定義する。
また、エラー列挙型のバリアントには、アプリにおける独自のエラー・コードを属性として付与して、エラー・コードでエラー内容を識別する。
さらに、マクロを定義して、エラー列挙型からエラー・コードを取得するメソッドを自動的に導出する。

```rust
#[derive(Debug, thiserror::Error, UseCaseError)]
pub enum RegisterUserError {
    /// 予期しないエラー
    #[error("Unexpected error: {0}")]
    #[use_case_error(error_code = 1000)]
    Unexpected(anyhow::Error),

    /// リポジトリ・エラー
    #[error("Repository error: {0}")]
    #[use_case_error(error_code = 1001)]
    Repository(anyhow::Error),

    /// パスワードが弱い
    #[error("Password is weak")]
    #[use_case_error(error_code = 2000)]
    WeakPassword,

    /// ユーザー名が既に登録されている
    #[error("User already exists: {0}")]
    #[use_case_error(error_code = 2001)]
    UserAlreadyExists(String),
}
```

`derive`属性に付与した`UseCaseError`が、エラー列挙型のバリアントのエラー・コードを返却するメソッドを実装するマクロである。
エラー・コードは、バリアントに`use_case_error`属性の`error_code`フィールドで定義する。

### コントローラー（ルーティング）層における実装

クリーン・アーキテクチャにおけるコントローラー（ルーティング層）で、`HttpResponse`に`From`トレイトを実装して、ユース・ケース層で定義したエラー型を`HttpResponse`に変換する。

```rust
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
```

これにより、コントローラー層で定義するリクエスト・ハンドラは、次のように実装できる。

```rust
pub async fn register_user(body: web::Json<RegisterUserRequestBody>) -> HttpResponse {
    let user = RegistrationUser {
        user_name: body.user_name.clone(),
        password: body.password.clone(),
    };

    match use_cases::register_user(user).await {
        Ok(_) => HttpResponse::Ok().finish(),
        // Fromトレイトの実装により、RegisterUserErrorをHttpResponseに変換
        Err(err) => err.into(),
    }
}
```

> ユース・ケース層で定義したエラー列挙型のバリアントに、クライアントに返すHTTPステータス・コードを属性として付与して、マクロでHTTPステータス・コードを取り扱う実装を自動で導出できるが、レイヤの責務に違反するため、`From`トレイトを実装した。

## ロギング

### 要求仕様

- デバッグ、情報、警告、エラーなどを区分してログを記録
- リクエストIDなど、それぞれのリクエストを追跡できるような情報とともにログに記録
- リクエスト・ハンドラの処理時間をログに記録する。

### 実装方針

 次のクレートを使用してロギングを実装する。

- [tracing](https://docs.rs/tracing/latest/tracing/index.html): 構造化されたイベント・ベースの診断情報を収集する計測フレームワーク
- [tracing-actix-web](https://docs.rs/tracing-actix-web/latest/tracing_actix_web/):  `actix-web`フレームワーク上に構築されたアプリケーションから遠隔測定データを収集するミドルウェアである[TracingLogger](https://docs.rs/tracing-actix-web/latest/tracing_actix_web/struct.TracingLogger.html)を提供
  - 自動的にリクエストにIDを付与して、リクエスト・パスとともにイベントを発行
- [tracing-bunyan-formatter](https://docs.rs/tracing-bunyan-formatter/latest/tracing_bunyan_formatter/): スパンへの出入り、イベントの作成時に、[Bunyan](https://github-com.translate.goog/trentm/node-bunyan?_x_tr_sl=en&_x_tr_tl=ja&_x_tr_hl=ja&_x_tr_pto=wapp)と互換性のあるレコードをJSON形式で発行
- [tracing-log](https://github.com/tokio-rs/tracing/tree/master/tracing-log): `log`クレートが提供するロギング・ファサードと一緒にトレースを使用する互換レイヤ
- [tracing-subscriber](https://github.com/tokio-rs/tracing/tree/master/tracing-subscriber): `tracing`クレートのサブスクライバを実装または構成するユーティリティ

### 実装概要

```rust
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

fn get_subscriber(name: String, default_log_level: String) -> impl Subscriber {
    // ログをフィルタする条件を環境変数から取得
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_log_level));

    // ログを購読するサブスクライバを構築
    let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // すべての`log`のイベントをサブスクライバにリダイレクト
    LogTracer::init().expect("failed to set log tracer");
    // 上記サブスクライバをデフォルトに設定
    set_global_default(subscriber).expect("failed to set subscriber");
}
```

リクエスト・ハンドラの実装を次に示す。

```rust
/// ユーザー登録リクエスト・ハンドラ
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
```

標準出力に出力されたログを次に示す。

<!-- cspell: disable -->
```json
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "start program",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:39.202877Z",
  "target": "web",
  "line": 14,
  "file": "web/src/main.rs"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "starting 8 workers",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:39.203475Z",
  "target": "actix_server::builder",
  "line": 240,
  "file": "/Users/xjr1300/.cargo/registry/src/index.crates.io-6f17d22bba15001f/actix-server-2.3.0/src/builder.rs"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "Tokio runtime found; starting in existing Tokio runtime",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:39.203593Z",
  "target": "actix_server::server",
  "line": 197,
  "file": "/Users/xjr1300/.cargo/registry/src/index.crates.io-6f17d22bba15001f/actix-server-2.3.0/src/server.rs"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "[HTTP REQUEST - START]",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:41.33571Z",
  "target": "tracing_actix_web::root_span_builder",
  "line": 41,
  "file": "/Users/xjr1300/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tracing-actix-web-0.7.10/src/root_span_builder.rs",
  "otel.kind": "server",
  "request_id": "20a3523e-ba06-47f3-b026-4d8be10a7ee7",
  "http.target": "/users",
  "http.flavor": "1.1",
  "http.method": "POST",
  "http.client_ip": "127.0.0.1",
  "http.scheme": "http",
  "http.host": "localhost:8080",
  "http.user_agent": "curl/8.4.0",
  "otel.name": "HTTP POST /users",
  "http.route": "/users"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "[REGISTER USER - START]",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:41.336353Z",
  "target": "web::routers",
  "line": 148,
  "file": "web/src/routers.rs",
  "otel.kind": "server",
  "request_id": "52e2e5af-c2a0-491e-bd1d-d49529a450d2",
  "http.target": "/users",
  "http.flavor": "1.1",
  "user_name": "kuro",
  "http.method": "POST",
  "http.client_ip": "127.0.0.1",
  "http.scheme": "http",
  "http.host": "localhost:8080",
  "http.user_agent": "curl/8.4.0",
  "otel.name": "HTTP POST /users",
  "http.route": "/users"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "[REGISTER USER USE CASE - START]",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:41.336467Z",
  "target": "web::use_cases",
  "line": 33,
  "file": "web/src/use_cases.rs",
  "otel.kind": "server",
  "request_id": "52e2e5af-c2a0-491e-bd1d-d49529a450d2",
  "http.target": "/users",
  "http.flavor": "1.1",
  "user_name": "kuro",
  "http.method": "POST",
  "http.client_ip": "127.0.0.1",
  "http.scheme": "http",
  "http.host": "localhost:8080",
  "http.user_agent": "curl/8.4.0",
  "otel.name": "HTTP POST /users",
  "http.route": "/users"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "[REGISTER USER USE CASE - END]",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:41.336568Z",
  "target": "web::use_cases",
  "line": 33,
  "file": "web/src/use_cases.rs",
  "otel.kind": "server",
  "request_id": "52e2e5af-c2a0-491e-bd1d-d49529a450d2",
  "http.target": "/users",
  "http.flavor": "1.1",
  "user_name": "kuro",
  "http.method": "POST",
  "http.client_ip": "127.0.0.1",
  "http.scheme": "http",
  "elapsed_milliseconds": 0,
  "http.host": "localhost:8080",
  "http.user_agent": "curl/8.4.0",
  "otel.name": "HTTP POST /users",
  "http.route": "/users"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "[REGISTER USER - END]",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:41.336751Z",
  "target": "web::routers",
  "line": 148,
  "file": "web/src/routers.rs",
  "otel.kind": "server",
  "request_id": "52e2e5af-c2a0-491e-bd1d-d49529a450d2",
  "http.target": "/users",
  "http.flavor": "1.1",
  "user_name": "kuro",
  "http.method": "POST",
  "http.client_ip": "127.0.0.1",
  "http.scheme": "http",
  "elapsed_milliseconds": 0,
  "http.host": "localhost:8080",
  "http.user_agent": "curl/8.4.0",
  "otel.name": "HTTP POST /users",
  "http.route": "/users"
}
{
  "v": 0,
  "name": "error_and_logging",
  "msg": "[HTTP REQUEST - END]",
  "level": 30,
  "hostname": "mac17.local",
  "pid": 42057,
  "time": "2024-03-20T06:57:41.337307Z",
  "target": "tracing_actix_web::root_span_builder",
  "line": 41,
  "file": "/Users/xjr1300/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tracing-actix-web-0.7.10/src/root_span_builder.rs",
  "otel.kind": "server",
  "request_id": "20a3523e-ba06-47f3-b026-4d8be10a7ee7",
  "http.target": "/users",
  "http.flavor": "1.1",
  "http.status_code": 200,
  "http.method": "POST",
  "http.client_ip": "127.0.0.1",
  "http.scheme": "http",
  "elapsed_milliseconds": 1,
  "http.host": "localhost:8080",
  "http.user_agent": "curl/8.4.0",
  "otel.status_code": "OK",
  "otel.name": "HTTP POST /users",
  "http.route": "/users"
}
```
