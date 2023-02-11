use server::Server;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub mod connection;
pub mod domain;
pub mod server;

#[tokio::main(flavor = "multi_thread")]
#[tracing::instrument]
async fn main() {
    let file_appender = RollingFileAppender::new(Rotation::NEVER, "/app", "speed.log");
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(file_appender)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("configure tracing");
    let mut server = Server::new();
    server.run().await.unwrap();
}
