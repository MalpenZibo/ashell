mod app;
mod bar;
mod config;
mod i18n;
mod modules;
#[cfg(test)]
mod testing;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    app::run();
}
