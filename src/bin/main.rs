use listenfd::ListenFd;
use rinha_de_backend::application::ServerData;
use rinha_de_backend::domain::account::Account;
use rinha_de_backend::infrastructure::server_impl::server::{match_routes, parse_http};
use rinha_de_backend::infrastructure::TransactionIPCRepository;
use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// FIXME: something weird is going, but if we change it to multithread
// account insert keeps overwriting one another despite de mutex
#[tokio::main()]
async fn main() {
    run().await
}

pub fn setup_ipcdb() -> TransactionIPCRepository {
    let repo = TransactionIPCRepository::init_pool();
    let accounts = [
        Account::new(1, 100_000),
        Account::new(2, 80_000),
        Account::new(3, 1_000_000),
        Account::new(4, 10_000_000),
        Account::new(5, 500_000),
    ];
    unsafe {
        repo.setup_db(&accounts);
    };
    repo
}

async fn run() {
    let mut listenfd = ListenFd::from_env();
    let socket = if let Some(listener) = listenfd.take_unix_listener(0).unwrap() {
        listener.set_nonblocking(true).unwrap();
        UnixListener::from_std(listener).unwrap()
        // TcpListener::from_std(listener).unwrap()
    } else {
        let socket_path = std::env::var("HOSTNAME")
            .map(|host| format!("/tmp/docker/{host}.sock"))
            .unwrap_or("/tmp/uepa.sock".to_string());
        let socket = UnixListener::bind(&socket_path).unwrap();

        let mut perm = fs::metadata(&socket_path).unwrap().permissions();
        perm.set_mode(0o777);
        fs::set_permissions(socket_path, perm).unwrap();

        socket
    };

    let data = ServerData {
        ipc_repo: setup_ipcdb(),
    };

    println!("Server is running!");
    loop {
        let (mut stream, _) = socket.accept().await.unwrap();
        // stream.set_nodelay(true).unwrap();

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
            }
        });
    }
}
