mod sequencer;
mod pyth_fetch;

use anyhow::Context;
use tokio::task::JoinSet;
use crate::pyth_fetch::fetch_price;
use crate::sequencer::Sequencer;

#[tokio::main]
async fn main() {
    // let args = SequencerArgs::parse();
    drop(run().await);
}

pub async fn run() -> anyhow::Result<()> {

    // println!("Hello, world!");

    let mut sequencer = Sequencer::new(
        "http://127.0.0.1:8080",
        "/tmp/signing_key.bin",
        None,
        None,
        "/tmp/queue.json",
        "/tmp/checkpoint.json",
        "/tmp/channel.json"
    ).context("Failed to initialize sequencer")?;

    /*
    tokio::task::spawn(async move {
        sequencer.run().await;
    });
    println!("Background sequenncer started");
    */

    // println!("Starting sequencer...");
    // sequencer.run().await;

    // Setup fetch price
    let pyth_base_url = "https://hermes.pyth.network/v2/updates/price/stream";
    let price_feed_eth_usdt = "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";

    let mut set = JoinSet::new();
    set.spawn(async move { sequencer.run().await });
    set.spawn(async move { fetch_price(pyth_base_url, price_feed_eth_usdt).await });

    while let Some(res) = set.join_next().await {
        match res {
            Ok(_) => {
                println!("Task finished");
                break;
            },
            Err(e) => {
                println!("Error: {:#?}", e);
                break;
            },
        }
    }

    Ok(())
}