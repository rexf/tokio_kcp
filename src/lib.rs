//! Library of KCP on Tokio

use chrono::Local;
pub use self::{
    config::{KcpConfig, KcpNoDelayConfig},
    listener::KcpListener,
    stream::KcpStream,
};

pub mod config;
pub mod listener;
pub mod session;
pub mod skcp;
pub mod stream;
pub mod utils;


pub fn us() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

pub fn init_logger() {
    use env_logger::{Builder, fmt::Formatter};
    use log::LevelFilter;
    use std::io::Write;
    use std::thread;
    use chrono::Local;
    use std::env;

    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "trace".to_string());

    Builder::new()
        .format(|buf: &mut Formatter, record| {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.6f");
            writeln!(buf, "{} [{}] ({}:{}) - {:?}| {}",
                     timestamp,
                     record.level(),
                     record.file().unwrap_or("unknown"),
                     record.line().unwrap_or(0),
                     thread::current().name().unwrap(),
                     record.args()
            )
        })
        .filter(None, log_level.parse().unwrap_or(LevelFilter::Debug))
        .init();
}
