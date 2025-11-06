mod config;
mod websocket;
mod error;
mod client;

use config::Config;
use websocket::Server;

#[tokio::main]
async fn main() {
    unsafe { std::env::set_var("RUST_LOG", "debug") }
    env_logger::init();

    let config = Config::from_file("configs/server.toml").expect("Failed to load config");
    let server = Server::new(config);
    let handle = server.listen().await.unwrap();
    handle.await.unwrap();
}
