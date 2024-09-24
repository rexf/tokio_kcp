use std::{net::SocketAddr, time::Duration};

use tokio_kcp::{init_logger, us, KcpConfig, KcpNoDelayConfig};
use tokio_kcp::KcpListener;

use byte_string::ByteStr;
use env_logger;
use log::{debug, error, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time,
};

fn config() -> KcpConfig {
    KcpConfig {
        mtu: 1400,
        nodelay: KcpNoDelayConfig::fastest(),
        wnd_size: (256, 256),
        session_expire: Some(Duration::from_secs(10)),
        flush_write: false,
        flush_acks_input: false,
        stream: false,
        allow_recv_empty_packet: false,
    }
}


#[tokio::main]
async fn main() {
    init_logger();

    let config = config();

    let server_addr = "127.0.0.1:3100".parse::<SocketAddr>().unwrap();
    info!("Binding to...{}", server_addr);
    let mut listener = KcpListener::bind(config, server_addr).await.unwrap();

    loop {
        info!("trying to accept connection...");
        let (mut stream, peer_addr) = match listener.accept().await {
            Ok(s) => s,
            Err(err) => {
                error!("accept failed, error: {}", err);
                time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        info!("accepted {}", peer_addr);

        tokio::spawn(async move {
            let mut buffer = [0u8; 8192];
            while let Ok(n) = stream.read(&mut buffer).await {
                if n > 0 {
                    stream.write_all(&buffer[..n]).await.unwrap();
                } else {
                    debug!("session closed?");
                    break
                }
            }

            debug!("client {} closed", peer_addr);
        });
    }
}
