use tokio::task::JoinHandle;
use tracing::subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn init_subscriber(name: &str) {
    // 将`log`中的记录导入`trace`中
    // 在`trace`中显示`actix-web`的日志
    LogTracer::init().expect("failed to set logger.");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // 日志输出到标准输出
    let formatting_layer = BunyanFormattingLayer::new(name.into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    subscriber::set_global_default(subscriber).expect("failed to set subscriber.");
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
