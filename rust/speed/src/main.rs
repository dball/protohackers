use server::Server;

pub mod connection;
pub mod domain;
pub mod server;

#[tokio::main(flavor = "multi_thread")]
#[tracing::instrument]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("configure tracing");
    let mut server = Server::new();
    server.run().await.unwrap();
}
