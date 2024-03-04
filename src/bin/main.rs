use heed::{DatabaseFlags, Env, EnvFlags, EnvOpenOptions};
use listenfd::ListenFd;
use redis::aio::ConnectionManager;
use redis::Client;
use rinha_de_backend::application::repositories::{HeedDB, TransactionLMDBRepository};
use rinha_de_backend::application::ServerData;
use rinha_de_backend::domain::account::Account;
use rinha_de_backend::infrastructure::server_impl::server::{match_routes, parse_http};
use rinha_de_backend::AnyResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    run().await
}

fn setup_lmdb() -> (Env, HeedDB) {
    let env = EnvOpenOptions::new().max_dbs(10).open("/tmp").unwrap();

    let mut rwtx = env.write_txn().unwrap();
    let db = env
        .database_options()
        .name("rinha.lmdb")
        .types()
        .create(&mut rwtx)
        .unwrap();
    db.clear(&mut rwtx).unwrap();
    rwtx.commit().unwrap();

    let repo = TransactionLMDBRepository {
        db: (env.clone(), db),
    };
    [
        Account::new(1, 100000),
        Account::new(2, 80000),
        Account::new(3, 1000000),
        Account::new(4, 10000000),
        Account::new(5, 500000),
    ]
    .iter()
    .for_each(|c| repo.save_account(c));

    (env, db)
}

async fn setup_redis() -> AnyResult<ConnectionManager> {
    let conn = ConnectionManager::new(Client::open("redis://localhost:6379")?).await?;
    Ok(conn)
}

async fn run() {
    console_subscriber::init();

    let mut listenfd = ListenFd::from_env();
    let socket = if let Some(listener) = listenfd.take_tcp_listener(0).unwrap() {
        listener.set_nonblocking(true).unwrap();
        // UnixListener::from_std(listener).unwrap()
        TcpListener::from_std(listener).unwrap()
    } else {
        TcpListener::bind("localhost:1337").await.unwrap()
    };

    let re_conn = setup_redis().await.unwrap();
    let data = ServerData {
        re_conn,
        lmdb_conn: setup_lmdb(),
    };

    println!("Server is running!");

    loop {
        let (mut stream, _) = socket.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();

        let data = data.clone();
        tokio::spawn(async move {
            // messages are not that long anyway
            let mut buf = [0; 1024];

            loop {
                let read_bytes = match stream.read(&mut buf).await {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                if read_bytes == 0 {
                    return;
                }

                let request = parse_http(&buf).unwrap();
                let response = match_routes(&data, request).await;
                let response = if response.is_left() {
                    response.unwrap_left()
                } else {
                    response.unwrap_right()
                };
                stream.write_all(&response.into_http()).await.unwrap();
                // stream.shutdown().await.unwrap();
            }
        });
    }
}
