mod sequencer;

use anyhow::Context;
use crate::sequencer::Sequencer;

#[tokio::main]
async fn main() {
    // let args = SequencerArgs::parse();
    drop(run().await);
}

pub async fn run() -> anyhow::Result<()> {
    println!("Hello, world!");

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
    println!("Starting sequencer...");
    sequencer.run().await;

    Ok(())
}