# actix-webのロギング

## logクレート

> `env_logger`クレートは、実行形式のプロジェクト(cargo new --bin ...)で使用することを想定しており、ライブラリのプロジェクトでは`log`クレートを使用する。

[log](https://crates.io/crates/log)クレートは、`trace`、`debug`、`info`、`warn`そして`error`の5つのマクロを提供する。
これらのマクロは、それぞれのログ・レベルでログ・レコードを発行する。

```rust
match result {
    Ok(success) => log::info!("Operation succeeded: {}", success);
    Err(error) => log::error!("Operation failed: {}", error);
};
```

`log`クレートは、発行先の二重性を処理するためにファサード・パターンを活用する。
`main`関数の最初で、`log::set_logger`関数を呼び出して、`log::Log`トレイトを実装したオブジェクトを渡すことで、ログが発行される。
もし、`set_logger`関数を呼び出さない場合、発行したログは破棄される。

## env_loggerクレート

`env_logger`クレーとは、`log::Log`トレイトを実装した`Logger`型を提供する。
この`Logger`型は、環境変数により実際に発行されるログ・レベルを変更する。

```rust
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // env_logger::Logger::init()は、`log::set_boxed_logger()`を呼び出し、`log_set_boxed_logger()`は
    // `log::set_logger()`を呼び出す。
    // `env_logger::Env::default()`は、環境変数`RUST_LOG`と`RUST_LOG_STYLE`を読み込み、
    // それそれ発行するログ・レベルとログの書式を設定する。
    // `env_logger::Env::default_filter_or()`は、`RUST_LOG`環境変数が設定されていない場合に、
    // 引数で与えたログ・レベルを使用する。
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // ...
}
```

上記の通りロガーを設定すると、次のログ・レコードなどが発行される。

```text
[2024-03-15T02:46:25Z INFO  actix_server::builder] starting 8 workers
[2024-03-15T02:46:25Z INFO  actix_server::server] Tokio runtime found; starting in existing Tokio runtime
[2024-03-15T02:46:37Z INFO  actix_web::middleware::logger] 127.0.0.1 "GET / HTTP/1.1" 200 9 "-" "curl/8.4.0" 0.000248
```

次を実行すると、デバッグ・レベル以上のログが発行される。

```sh
RUST_LOG=debug cargo run
```

次を実行すると、プロジェクトが依存しているクレートから発行されたログを破棄する。
ただし、`package_name`には、パッケージ名の`-`を`_`に置き換えて指定する。

```sh
RUST_LOG=package_name cargo run
```

## actix-webのLoggerミドルウェア

`actix-web`は`Logger`ミドルウェアを提供しており、`Logger`ミドルウェアはすべての受信リクエストのログ・レコードを発行する。
ログは、ファイル、ターミナルまたはHTTPを介してリモートのサーバーなどに発行される。

## tracingクレート

[tracing](https://docs.rs/tracing/latest/tracing/)

### 概要

`tracing`クレートは、構造化され、イベント・ベースの診断情報を収集するためにRustプログラムを計測するためのフレームワークである。

`tokio`のような非同期システムにおいて、従来からあるログ・メッセージを解釈することは、より複雑になることがよくある。
それぞれのタスクは同じスレッドで多重化されるため、関連するイベントやログ行が混じるため、ロジックの流れを追跡することを困難にする。
`tracing`は、ライブラリとアプリケーションが*一時性*と*因果関係*に関する追加情報を含む構造化されたイベントを記録するために、ロギング・スタイルの診断を拡張する。
ログ・メッセージと異なり、`tracing`の`span`は開始と終了時間を持ち、おそらく実行フローによって開始そして終了され、同様な`span`のネストされたツリーに存在する可能性がある。
加えて、`tracing`の`span`は構造化されており、テキストのメッセージと同様に型付けられたデータを記録する能力がある。

`tracing`クレートは、追跡データを発行するために、ライブラリとアプリケーションを計測するために必要なAPIを提供する。

### コア・コンセプト

`tracing`のコア・コンセプトは*スパン*、*イベント*そして*サブスクライバ*の構成である。
順番にこれらを説明する。

#### スパン

[span](https://docs.rs/tracing/latest/tracing/span/index.html)

プログラムを介して実行フローを記録するために、`tracing`は[span](https://docs.rs/tracing/latest/tracing/span/index.html)の概念を導入している。
瞬間を表現するログ行と異なり、スパンは開始と終了を持つ時間の期間を表現する。
プログラムがコンテキスト内で実行を開始するかユニット・オブ・ワークを実行するとき、スパンはそのコンテキストのスパンに入り、そのコンテキストの実行が終了したとき、そのスパンを出る。
すレッドが現在実行しているスパンは、スレッドの現在のスパンと呼ばれる。

```rust
use tracing::{span, Level};
let span = span!(Level::TRACE, "my_span");
// `enter`はRAIIガードを返し、ドロップしたとき、そのスパンを出る。
// これは現在のレキシカル・スコープのスパン内にいることを示す。
let _enter = span.enter();
// `my_span`のコンテキスト内で任意の作業を実行する。
```

**注意**: `async/await`構文を使用する非同期コードにおいて、もし、返されたドロップ・ガードが待機ポイントを超えて保持された場合、`Span::enter`は不正確な追跡を生成する可能氏がある。
> 詳細は[`enter`](https://docs.rs/tracing/latest/tracing/struct.Span.html#in-asynchronous-code)メソッドのドキュメントを参照すること。

```rust
async fn my_async_function() {
    let span = info_span!("my_async_function");

    // 警告: このスパンはこのガードがドロップされるまで入ったままである・・・。
    let _enter = span.enter();
    // ...but the `await` keyword may yield, causing the
    // runtime to switch to another task, while remaining in
    // this span!
    // ・・・しかし、このスパン内に残っている一方で、`await`キーワードは、ランタイムを他のタスクに切り替える結果を生み出す。
    some_other_async_function().await
}
```

`Span::enter`によって返されたドロップ・ガードは、それがドロップされたときにスパンを終了する。
非同期関数または非同期ブロックが`.await`ポイントで生み出したとき、現在のスコープは*終了される*が、非同期ブロックは最終的に待機点から実行を再開するため、そのスコープ内の値は*ドロップされない*。
これは、開始されたスパン内に*残っている*間に*他の*タスクが実行を開始したことを意味する。
これは不正確な追跡となる。

非同期コードで`Span::enter`を使用する代わりに、次が好ましい。

非同期ブロックまたは関数内のコードの非同期部分でスパンを開始するために、[Span::in_scope](https://docs.rs/tracing/latest/tracing/struct.Span.html#method.in_scope)を使用することが望ましい。
`in_scope`は同期クロージャーを受け取り、そのクロージャーが戻った時にスパンを終了するため、そのスパンは常に次の待機店の前に終了させられる。

```rust
async fn my_async_function() {
    let span = info_span!("my_async_function");

    // span.enter()していないことに注意
    let some_value = span.in_scope(|| {
        // このスパン内で任意の同期コードを実行する。
    });

    // 次は大丈夫である。
    // 待機点に到着する前にスパンはすでに終了されている。
    some_other_async_function(some_value).await;
}
```

非同期コードを計測するために、`tracing`はフューチャー（非同期関数またはブロック）にスパンを取り付ける[Future::instrument](https://docs.rs/tracing/latest/tracing/trait.Instrument.html)コンビネーターを提供する。

`Instrument`は非同期関数内の非同期ブロック内で使用できる。

```rust
use tracing::Instrument;

async fn my_async_function() {
    let span = info_span!("my_async_function");
    // span.enter()していないことに注意
    async move {
        // これは正しい。
        // もしここで生成した場合、スパンが終了して、再開した時にスパンに戻る。
        some_other_async_function().await;

        // スパン内の他の非同期コード
        // [...]
    }
    // スパンを持つ非同期ブロックを計測
    .instrument(span)
    // そして、非同期コードが終了するまで待機
    .await;
}
```

呼び出し側で非同期関数を呼び出しを計測するためにも使用できる。

```rust
use tracing::Instrument;

async fn my_async_function() {
    let some_value = some_other_async_function()
        .instrument(debug_span!("some_other_async_function"))
        .await;
    // [...]
}
```

`#[instrument]`属性マクロは、非同期関数で使用するとき、自動で正確なコードを生成できる。

```rust
#[tracing::instrument(level = "info")]
async fn my_async_function() {
    // これは正しい。
    // もしここで生成した場合、スパンが終了して、再開した時にスパンに戻る。
    some_other_async_function().await;
    // [...]
}
```

#### イベント

[Event](https://docs.rs/tracing/latest/tracing/event/struct.Event.html)

`Event`は時間内の*瞬間*を表現する。
イベントは追跡が記録されている間に発生したことを示す。
`Event`は構造化されていないロギング・コードによって発行されたログ・レコードと互換性があるが、典型的なログ行と異なり、`Event`はスパンの文脈内で発生する可能性がある。

```rust
use tracing::{event, span, Level};

// 任意のスパンのコンテキストの外でイベントを記録する。
event!(Level::INFO, "something happened");

let span = span!(Level::INFO, "my_span");
let _guard = span.enter();

// 「my_span」でイベントを記録する。
event!(Level::DEBUG, "something happened inside my_span");
```

一般的に、イベントは、与えられたステータス・コードで返されたリクエストや、キューから*n*個の新しいアイテムが取得されたなど、スパン*内*の時点を表現するために使用されるべきである。

#### サブスクライバ

[Subscriber](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html)

`Span`や`Event`が発生した場合、それらは`Subscriber`トレイトの実装によって記録または集約される。
`Subscriber`は、`Event`が発生したときと、`Span`が開始または終了したときに、通知される。
これらの通知は次の`Subscriber`トレイトメソッドによって表現される。

* [event](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html#tymethod.event): `Event`が発生したときに呼び出される。
* [enter](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html#tymethod.enter): 実行が`Span`を開始したときに呼び出される。
* [exit](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html#tymethod.exit): 実行が`Span`を終了したときに呼び出される。

加えて、サブスクライバは、`Span`または`Even`それぞれを説明する[メタデータ](https://docs.rs/tracing/latest/tracing/struct.Metadata.html)に基づいて、サブスクライバが受け取った通知をフィルタするために[enabled](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html#tymethod.enabled)関数を実装している可能性がある。
もし、`Subscriber::enabled`の呼び出しが、与えられたメタデータのセットによって`false`を返した場合、その`Subscriber`は対応する`Span`または`Event`に関して通知しない。
性能の理由から、`true`を返すことによって与えられたメタデータのセットに興味を持つ現在活動しているサブスクライバが存在しない場合、対応する`Span`または`Event`は決して構築されない。

### 使用方法

最初に`Cargo.toml`に次を追加する。

```toml
[dependencies]
tracing = "0.1"
```

#### スパンとイベントの記録

スパンとイベントはマクロを使用して記録される。

##### スパン

[span!](https://docs.rs/tracing/latest/tracing/macro.span.html)マクロは、スパンを記録するために使用される[Span](https://docs.rs/tracing/latest/tracing/struct.Span.html)構造体に拡張する。
[Span::enter](https://docs.rs/tracing/latest/tracing/struct.Span.html#method.enter)メソッドは[RAII](https://github.com/rust-unofficial/patterns/blob/main/src/patterns/behavioural/RAII.md)ガード・オブジェクトを返してそのスパンが開始されたこと、そのガードオブジェクトはドロップしたときスパンを終了したことを記録する。

```rust
use tracing::{span, Level};
// 「my_span」と名付けた新しいスパンをトレース・ログ・レベルで構築する。
let span = span!(Level::TRACE, "my_span");

// スパンを開始して、ガード・オブジェクトが返される。
let _enter = span.enter();

// ガードがドロップされる前に発生したトレース・一ベントが、スパン内で発生する。

// ガードのドロップはスパンを終了する。
```

[#[instrument]](https://docs.rs/tracing-attributes/latest/tracing_attributes/attr.instrument.html)属性は関数に`tracing`のスパンを追加することを容易にする。
`#[instrument]`で注釈された関数は、その関数が呼びだされるたびに、関数の名前を持つスパンを作成して開始する。
そして、その関数に与えられた引数が`fmt::Debug`を使用してフィールドとして記録される。

```rust
use tracing::{Level, event, instrument};

#[instrument]
pub fn my_function(my_arg: usize) {
    // このイベントは`my_arg`フィールドを持つ`my_function`と名付けられたスパン内に記録される。
    event!(Level::INFO, "inside my_function!");
    // [...]
}
```

`tracing`のサポートがビルトインされていない、また`#[instrument]`属性を適用できない関数（外部のクレートから来たような）のために、`Span`構造体はスパン内に同期コードを容易に包むために使用される`in_scope`メソッドがある。

```rust
using tracing::info_span;

let json = info_span!("json.parse").in_scope(|| serde_json::from_slice(&buf))?;
```

##### イベント

`Event`は`event!`マクロを使用して記録される。

```rust
use tracing::{event, Level};
event!(Level::INFO, "something has happened!");
```

##### マクロの使用

`span!`や`event!`マクロと同様に`#[instrument]`属性は、いくつかの例外があるが、とても似た構文を使用する。

###### 属性の構成

両方のマクロはスパンまたはイベントの詳細さを指定する`Level`を要求する。
任意で、[target](https://docs.rs/tracing/latest/tracing/struct.Metadata.html#method.target)と[parent](https://docs.rs/tracing/latest/tracing/span/struct.Attributes.html#method.parent)のスパンは、上書きされる可能性がある。
もしターゲットと親のスパンが上書きされなかった場合、それらはマクロが呼び出されたモジュール・パスと現在のスパン（サブスクライバによって決定される）がそれぞれデフォルトになる。

```rust
span!(target: "app_spans", Level::TRACE, "my span");
event!(target: "app_events", Level::INFO, "something has happened!");
```

```rust
let span = span!(Level::TRACE, "my_span");
event!(parent: &span, Level::INFO, "something has happened!");
```

またスパン・マクロはレベルの後で文字列リテラルを受け取り、それは（上記同様）スパンの名前を設定する。
イベント・マクロの場合、イベントの名前は`name:`記述子を使用することで上書きされる（デフォルトは`event file:line`）。

```rust
span!(Level::TRACE,"my span");
event!(name: "some_info", Level::INFO, "something has happened!");
```

##### フィールドの記録

スパンとイベントの構造化されたフィールドは`field_name = field_value`構文を使用することで指定できる。
フィールドは間まで分離される。

```rust
// 2つのフィールドを持つイベントを記録する。
// - "answer"は値42を持つ。
// - "question"は値"life, the universe, and everything"を持つ。
event!(Level::INFO, answer = 42, question = "life, the universe, and everything");
```

簡略表記として、ローカル変数は、[構造体の初期化](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#using-the-field-init-shorthand-when-variables-and-fields-have-the-same-name)と同様に、割り当てなしでフィールドの値として利用できる。

```rust
let user = "ferris";

span!(Level::TRACE, "login", user);
// 次と等価である。
span!(Level::TRACE, "login", user = user);
```

フィールド名はドットを含めることができるが、それらによって終了されるべきではない。

```rust
let user = "ferris";
let email = "ferris@rust-lang.org";
span!(Level::TRACE, "login", user, user.email = email);
```

フィールド名にドットを含めることができるため、簡略表記として、ローカルな構造体のフィールドはローカル変数を使用することで使用できる。

```rust
let user = User {
    name: "ferris",
    email: "ferris@rust-lang.org",
};
// スパンは`user.name = "ferris"`フィールドと`user.email = "ferris@rust-lang.org"`フィールドを持つ。
span!(Level::TRACE, "login", user.name, user.email);
```

Rustの識別子でない、またはRustの予約語の名前を持つフィールドは、クォートされた文字列リテラルを使用して作成される。
しかし、これはローカル変数の簡略表記として使用できない。

```rust
// Rustの識別子でない名前のフィールドを持つイベントを記録する。
// - "guid:x-request-id": `:`を含み、値"abcdef"を持つ。
// - "type": 予約語で、値"request"を持つ。
span!(Level::TRACE, "api", "guid:x-request-id" = "abcdef", "type" = "request");
```

また、定数の値が定数の名前でなくフィールド名として使用されることを示すためには、定数を波括弧で囲わなければならない。

```rust
const RESOURCE_NAME: &str = "foo";
// このスパンは`foo = "some_id"`フィールドを持つ。
span!(Level::TRACE, "get", { RESOURCE_NAME } = "some_id");
```

`?`記号は`fmt::Debug`実装を使用してフィールドを記録する必要があることを示す簡略表記である。

```rust
#[derive(Debug)]
struct MyStruct {
    field: &'static str,
}

let my_struct = MyStruct {
    field: "Hello world!",
};

// `my_struct`はその`fmt::Debug`実装を使用して記録される。
event!(Level::TRACE, greeting = ?my_struct);
// 次は上記と同じである。
event!(Level::TRACE, greeting = tracing::field::debug(&my_struct));
```

`%`記号も同様に動作するが、値がその`fmt::Display`実装で記録される必要があることを示す。

```rust
// `my_struct.field`はその`fmt::Display`実装を使用して記録される。
event!(Level::TRACE, greeting = %my_struct.field);
// 次は上記と同じである。
event!(Level::TRACE, greeting = tracing::field::display(&my_struct.field));
```

また、`%`と`?`記号はローカル変数の簡略表記に使用できる。

```rust
// `my_struct.field`はその`fmt::Display`実装を使用して記録される。
event!(Level::TRACE, %my_struct.field);
```

加えて、スパンは[Empty](https://docs.rs/tracing/latest/tracing/field/struct.Empty.html)という特別な値でフィールドを宣言する可能性があり、それはそのフィールドの値が現在存在しないが、後で記録される可能性があることを示す。

```rust
use tracing::{trace_span, field};

// 値に"hello world"を持つ`greetingと、値がない`parting`の2つのフィールドを持つスパンを作成する。
let span = tracing_span!("my_span", greeting = "hello world", parting = field::Empty);

// [...]

// `parting`の値を記録する。
span.record("parting", &"goodby world");
```

最後に、またイベントは、イベントのキーと値の後に、[書式文字列](https://doc.rust-lang.org/nightly/alloc/fmt/index.html#usage)と（任意の）引数で構成される、人が読みやすいメッセージを含めることができる。

```rust
let question = "the ultimate question of life, the universe, and everything";
let answer = 42;
// 次のフィールドを含めてイベントを記録する。
// - `question.answer`は値42を持つ。
// - `question.tricky`は値`true`を持つ。
// - "message"は値"the answer to the ultimate question of line, the universe, and everything is 42."を持つ。
event!(
    Level::Debug,
    question.answer = answer,
    question.tricky = true,
    "the answer to {} is {}.", question, answer
);
```

この方法で指定した初期化されたメッセージは、デフォルトで何も割り当てられない。

##### マクロによる簡略表記

`tracing`は、前もって設定された詳細レベルを持ついくつかのマクロを提供する。
`trace!`、`debug!`、`info!`、`warn!`そして`error!`は`event!`マクロと同様に振る舞うが、`Level`引数がすでに指定されている一方で、
`trace_span!`、`debug_span!`、`info_span!`、`warn_span!`そして`error_span!`マクロは同じだが、`span!`マクロに対応する。

これらは簡略表記と`log`クレートとの互換性の両方を意図している。

##### ライブラリについて

ライブラリは`tracing`クレートのみにリンクして、下流の利用者にとって有益な情報を記録するために提供されたマクロを使用するべきである。

##### 実行形式について

トレース・イベントを記録するために、実行形式は`tracing`と互換性がある`Subscriber`実装を使用しなければならない。
`Subscriber`は、標準出力にトレース・データをログ出力するような、トレース・データを収集する方法を実装している。

このライブラリは`Subscriber`実装を含んでおらず、これらは[他のクレート](https://docs.rs/tracing/latest/tracing/#related-crates)によって提供されている。

サブスクライバを使用する最も簡単な方法は、[set_global_default](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html)関数を呼び出すことである。

```rust
extern crate tracing;

let my_subscriber = FooSubscriber::new();
tracing::subscriber::set_global_default(my_subscriber)
    .expect("setting tracing default failed");
```

> **警告**: 一般的に、ライブラリは`set_global_default()`を呼び出すべきではない。
> それをすると、ライブラリに依存する実行形式が、後でライブラリのデフォルトを設定するときに衝突を起こす。

このサブスクライバは、`log`クレートのロガーの設定と同様に、プログラムの残りの間、すべてのスレッドのデフォルトとして使用される。

加えて、デフォルト・サブスクライバは[with_default](https://docs.rs/tracing/latest/tracing/subscriber/fn.with_default.html)関数を使用することで設定することができる。
これは、クロージャーの最後で終了するコンテキス内で実行するコードを表現するクロージャを使用する`tokio`パターンに従う。

```rust
let my_subscriber = FooSubscriber::new();
tracing::subscriber::with_default(my_subscriber, || {
    // このクロージャー内、またはクロージャーが呼び出す関数によって生成されたトレース・イベントは、
    // `my_subscriber`によって収集される。
})
// これ以降のトレース・イベントは、`my_subscriber`によって収集されない。
```

この方法は、プログラムの異なるコンテキストにある複数のサブスクライバによってトレース・データを収集させる。

上書きは現在実行中のスレッドにのみ適用されることに注意しなければならず、他のスレッドには`with_default`による変更を知らない。

サブスクライバのコンテキスト外で生成されたトレース・イベントは収集されない。

サブスクライバが設定されると、`tracing`クレートのマクロを使用して実行形式に計測点を追加できる。
