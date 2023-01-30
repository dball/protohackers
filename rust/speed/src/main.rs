use server::Server;

pub mod connection;
pub mod domain;
pub mod server;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut server = Server::new();
    server.run().await.unwrap();
}
