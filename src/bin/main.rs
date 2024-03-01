use deadpool_postgres::{ManagerConfig, Pool, RecyclingMethod, Runtime};
use listenfd::ListenFd;
use redis::aio::ConnectionManager;
use redis::Client;
use rinha_de_backend::application::cache::AccountCache;
use rinha_de_backend::application::repositories::TransactionRepository;
use rinha_de_backend::application::ServerData;
use rinha_de_backend::infrastructure::server_impl::server::{match_routes, parse_http};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_postgres::NoTls;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    run().await
}

async fn setup_pgsql() -> Pool {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some("rinhabackend".to_string());
    cfg.host = Some("localhost".to_string());
    cfg.user = Some("postgres".to_string());
    // cfg.password = "inha";
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
}

async fn setup_redis(pool: &Pool) -> ConnectionManager {
    let conn = ConnectionManager::new(Client::open("redis://localhost:6379").unwrap())
        .await
        .unwrap();

    let repo = TransactionRepository { conn: pool.clone() };

    let cache = AccountCache {
        re_conn: conn.clone(),
    };
    for acc in repo.get_accounts().await {
        cache.save_account(acc.id, &acc, None).await;
    }
    conn
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

    let pg_pool = setup_pgsql().await;
    let re_conn = setup_redis(&pg_pool).await;

    let data = ServerData { re_conn, pg_pool };

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
