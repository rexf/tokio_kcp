use env_logger;
use futures::future::poll_fn;
use serde::{Serialize, Deserialize, forward_to_deserialize_any};
use bincode;
use log::{debug, error, info, warn};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use std::{net::SocketAddr, str};
use std::io::Read;
use chrono::Local;
use futures::ready;
use tokio::io::{stdin, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
use tokio::select;
use tokio::sync::{Mutex, MutexGuard, TryLockError};
use tokio_kcp::KcpStream;
use tokio_kcp::{init_logger, us, KcpConfig, KcpNoDelayConfig};

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

async fn poll_read_kcp_stream(stream: Arc<Mutex<KcpStream>>) {
    let mut buffer = [0u8; 2048];

    let mut sum_rtt = 0;
    let mut n_rtt = 0;

    loop {
        // Poll the stream to read
        tokio::task::yield_now().await;

        // poll_fn(|cx: &mut Context<'_>| {
        match stream.lock().await {
            Ok(mut stream_lock) => {
                let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
                match stream_lock.poll_recv(cx, &mut buffer) {
                    Poll::Ready(Ok(n)) => {
                        if n == DATA_SIZE {
                            match bincode::deserialize::<Data>(&mut buffer) {
                                Ok(data) => {
                                    // info!("n={} data = {:?}", n, data);
                                    let rtt = Local::now().timestamp_micros() - data.timestamp;
                                    sum_rtt += rtt;
                                    n_rtt += 1;
                                }
                                Err(errd) => {
                                    debug!("Error deserialize data")
                                }
                            }
                        }
                        // Poll::Ready(Ok(()))
                    }
                    Poll::Ready(Err(e)) => {
                        debug!("Error, e={}", e);
                        // Poll::Ready(Err(e))
                    }
                    Poll::Pending => {
                        debug!("Pending...");
                        // Poll::Pending
                    }
                }
            }
            Err(er) => {
                debug!("lock failed. er={}", er);
                // Poll::Ready(Ok(()))
            }
        }
        if (n_rtt % 5000) == 0 {
            info!("average: {}, sum: {}, n: {}", sum_rtt as f64 / n_rtt as f64, sum_rtt, n_rtt);
        }
    }
    // )
    // .await
    // .unwrap_or_else(|e| {
    //     error!("Failed to read from stream: {:?}", e);
    // });


    // }
}

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    seq: i64,
    timestamp: i64,
    data1: i64,
    data2: i64,
}

const DATA_SIZE: usize = std::mem::size_of::<Data>();

async fn send_alive_periodically(stream: Arc<Mutex<KcpStream>>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut stream_lock = stream.lock().await;
        stream_lock.write_all(b"alive").await.unwrap();
    }
}


#[tokio::main]
async fn main() {
    init_logger();

    let config = config();

    let server_addr = "127.0.0.1:3100".parse::<SocketAddr>().unwrap();
    info!("Connecting to... {}", server_addr);

    let stream = KcpStream::connect_with_conv(&config, 3102, server_addr).await.unwrap();
    let stream = Arc::new(Mutex::new(stream));


    let a = Arc::clone(&stream);
    tokio::spawn(async move {
        poll_read_kcp_stream(a).await;
    });

    // let b = Arc::clone(&stream);
    // tokio::spawn(async move {
    //     send_alive_periodically(b).await;
    // });

    // let mut buffer = [0u8; 8192];
    // let mut i = stdin();

    for seqnum in 1..=100_000 {
        let data = Data {
            seq: seqnum,
            timestamp: Local::now().timestamp_micros(),
            data1: 123456789,
            data2: 987654321,
        };
        let encoded = bincode::serialize(&data).unwrap();
        // let buffer = encoded.as_slice();

        let mut retry = true;
        while retry {
            let mut stream_lock = stream.lock().await;

            match stream_lock.write_all(&encoded).await {
                Ok(_) => {
                    retry = false;
                }

                Err(e) => {
                    error!("connection write err={}", e);
                }
            }
        }
    }
    info!("Enter to exit.");
    let mut buffer = [0u8; 8192];
    let mut i = stdin();
    i.read(&mut buffer).await;
}
