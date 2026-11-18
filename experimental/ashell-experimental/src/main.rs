mod app;
mod bar;
mod config;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    app::run();
}
