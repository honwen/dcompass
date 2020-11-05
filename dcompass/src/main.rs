mod worker;

use crate::worker::worker;
use anyhow::Result;
use droute::filter::Filter;
use futures::future::join_all;
use log::*;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use tokio::fs::File;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio_compat_02::FutureExt;

#[tokio::main]
async fn main() -> Result<()> {
    use clap::{load_yaml, App};

    let yaml = load_yaml!("args.yaml");
    let m = App::from(yaml).get_matches();

    let (filter, addr, num_workers, verbosity) = match m.value_of("config") {
        Some(c) => {
            let mut file = File::open(c).await?;
            let mut config = String::new();
            file.read_to_string(&mut config).await?;
            Filter::new(&config).compat().await?
        }
        None => {
            Filter::new(include_str!("../../droute/configs/default.json"))
                .compat()
                .await?
        }
    };

    SimpleLogger::new().with_level(verbosity).init()?;

    let filter = Arc::new(filter);
    // Bind an UDP socket
    let socket = Arc::new(UdpSocket::bind(addr).await?);

    let mut handles = vec![];

    for i in 1..=num_workers {
        let socket = socket.clone();
        let filter = filter.clone();

        handles.push(tokio::spawn(async move {
            loop {
                let socket = socket.clone();
                let filter = filter.clone();

                match worker(filter, socket, i - 1).await {
                    Ok(_) => (),
                    Err(e) => warn!("Handling query failed: {}", e),
                }
            }
        }));
    }

    join_all(handles).await;

    Ok(())
}
