use tokio;
use std::io;

#[macro_use]
extern crate derive_error;

mod server;
mod client;
mod common;

use crate::common::config::{Args, Mode};

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = Args::new()
        .parse()
        .map_err(|reason| io::Error::new(
            io::ErrorKind::InvalidInput, 
            reason
        ))?;

    match config.mode {
        Mode::Client => client::run(config).await,
        Mode::Server => server::run(config).await
    }
}
