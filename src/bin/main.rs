use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use rinha_de_backend::server_impl::server::process_server_request;

#[tokio::main]
async fn main() {
    run().await
}

async fn run() {
    let socket = TcpListener::bind("0.0.0.0:1337").await.unwrap();

    loop {
        let (mut socket, other) = socket.accept().await.unwrap();
        // socket.set_nodelay()

        tokio::spawn(async move {
            let mut buf = [0; 2048];

            // In a loop, read data from the socket and write the data back.
            loop {
                let request = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => process_server_request(&mut buf, n),
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                eprintln!("{:?}", request);
                socket.shutdown().await;

                // // Write the data back
                // if let Err(e) = socket.write_all(&buf[0..n]).await {
                //     eprintln!("failed to write to socket; err = {:?}", e);
                //     return;
                // }
            }
        });
    }
}
