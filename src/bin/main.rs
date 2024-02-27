use redis::Client;
use rinha_de_backend::application::ServerData;
use rinha_de_backend::infrastructure::server_impl::server::{match_routes, parse_http};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    run().await
}

async fn run() {
    let socket = TcpListener::bind("0.0.0.0:1337").await.unwrap();
    let redis = Client::open("redis://0.0.0.0:1000").unwrap();
    let data = ServerData {
        redis: Arc::new(redis),
    };

    loop {
        let (mut socket, other) = socket.accept().await.unwrap();

        let data = data.clone();
        tokio::spawn(async move {
            // messages are not that long anyway
            let mut buf = [0; 1024];

            loop {
                let read_bytes = match socket.read(&mut buf).await {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                if read_bytes == 0 {
                    return;
                }

                eprintln!("one request, read bytes: {}", read_bytes);

                let request = parse_http(&buf).unwrap();
                let response = match_routes(&data, request);

                socket
                    .write_all(&response.left().unwrap().into_http())
                    .await
                    .unwrap();

                // shutdown connections immediately since gatling doesn't have keep-alive connection by default
                // socket.shutdown().await.unwrap();
            }
        });
    }
}
