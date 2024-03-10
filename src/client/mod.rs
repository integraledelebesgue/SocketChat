use std::io;
use tokio::sync::mpsc::unbounded_channel;

use crate::common::config::Config;

pub mod driver;
pub mod interface;
pub mod parser;

pub async fn run(config: Config) -> io::Result<()> {
    let (request_tx, request_rx) = unbounded_channel();
    let (response_tx, response_rx) = unbounded_channel();

    let driver = driver::run(config, request_rx, response_tx);
    let cli = interface::run(response_rx, request_tx);

    tokio::try_join!(driver, cli).map(|_| ())
}
