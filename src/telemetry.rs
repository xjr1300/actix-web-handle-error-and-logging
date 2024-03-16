use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber(name: String, default_log_level: String) -> impl Subscriber {
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

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // すべての`log`のイベントをサブスクライバにリダイレクト
    LogTracer::init().expect("failed to set log tracer");
    // 上記サブスクライバをデフォルトに設定
    set_global_default(subscriber).expect("failed to set subscriber");
}
