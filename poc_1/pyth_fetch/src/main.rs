use std::str::FromStr;
use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let price_latest = match args.command {
        Commands::Fetch => {
            let filter = PriceLatestFilter {
                ids: args.price_feed,
            };
            let hermes_url = Url::from_str("https://hermes.pyth.network")?;
            let price_latest_url = hermes_url.join("/v2/updates/price/latest")?;

            let client = reqwest::Client::new();
            println!("Fetching from {:?}", price_latest_url);
            let price_latest_res = client
                .get(price_latest_url.as_str())
                .query(&filter)
                .send()
                .await?;
            println!("DONE.");
            let price_latest: PriceUpdate = price_latest_res.json().await?;
            println!("price_latest: {:#?}", price_latest);
            // println!("price_latest: {:?}", price_latest);

            let fs = std::fs::File::create(args.price_json)?;
            serde_json::to_writer_pretty(fs, &price_latest)?;

            price_latest
        }
        Commands::Read => {
            let price_latest: PriceUpdate = serde_json::from_reader(std::fs::File::open(args.price_json)?)?;
            // println!("price_latest: {:?}", price_latest);
            println!("price_latest: {:#?}", price_latest);
            price_latest
        }
    };

    let bytes = &*hex::decode(price_latest.binary.data[0].as_str())?;
    let result = bytes
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(",");

    println!("pyth payload (ready for spel): {}", result);

    Ok(())
}

#[derive(Parser, Debug)]
#[command(name = "pyth_fetch_validate")]
#[command(version = "1.0")]
#[command(about = "Dev utilities", long_about = None)]
pub struct Args {
    #[arg(long = "hermes-url", default_value = "https://hermes.pyth.network")]
    pub hermes_url: String,
    // default value: ETH-USD price feed
    #[arg(short, long = "price-feed", default_value = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace")]
    pub price_feed: String,
    #[arg(long = "price-json", default_value = "price_latest.json")]
    pub price_json: String,
    #[arg(short, long = "guardians-json", default_value = "../near_sc_extract/guardians.json")]
    pub guardians_json: String,


    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Fetch, // Fetch from Hermes API - write result into args.price-json
    Read,  // Read json file written by Fetch
}

// Filter for Hermes API

#[derive(Debug, Serialize)]
struct PriceLatestFilter {
    #[serde(rename = "ids[]")]
    ids: String,
}

// PriceUpdate struct (return from Pyth price latest REST API - Hermes API)

#[derive(Debug, Serialize, Deserialize)]
struct PriceUpdate {
    binary: BinaryUpdate,
    parsed: Vec<ParsedPriceUpdate> // TODO: Should be an Vec<Option<...>> or a Option<Vec<...>>?
}

#[derive(Debug, Serialize, Deserialize)]
struct BinaryUpdate {
    data: Vec<String>,
    encoding: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParsedPriceUpdate {
    ema_price: RpcPrice,
    id: String, // RrcPriceIdentifier,
    metadata: RpcPriceFeedMetadataV2,
    price: RpcPrice,
}

#[derive(Serialize, Deserialize)]
struct RpcPrice {
    conf: String,
    expo: i32,
    price: String,
    publish_time: i64,
}

impl std::fmt::Debug for RpcPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        if f.alternate() {
            // Pretty print logic (i.e., {:#?})
            let date_str = DateTime::<Utc>::from_timestamp(self.publish_time, 0).unwrap();
            f.debug_struct("RpcPrice")
                .field("conf", &self.conf)
                .field("expo", &self.expo)
                .field("price", &self.price)
                .field("publish_time", &date_str.to_string()) // Pass our formatted string instead of the i64
                .finish()
        } else {
            // Standard print logic
            f.debug_struct("RpcPrice")
                .field("conf", &self.conf)
                .field("expo", &self.expo)
                .field("price", &self.price)
                .field("publish_time", &self.publish_time) // Pass our formatted string instead of the i64
                .finish()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RpcPriceFeedMetadataV2 {
    prev_publish_time: i64,
    proof_available_time: i64,
    slot: i64,
}

impl std::fmt::Debug for RpcPriceFeedMetadataV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            // Pretty print logic (i.e., {:#?})
            let prev_date_str = DateTime::<Utc>::from_timestamp(self.prev_publish_time, 0).unwrap();
            let proof_date_str = DateTime::<Utc>::from_timestamp(self.proof_available_time, 0).unwrap();
            f.debug_struct("RpcPriceFeedMetadataV2")
                .field("prev_publish_time", &prev_date_str.to_string())
                .field("proof_available_time", &proof_date_str)
                .field("slot", &self.slot)
                .finish()
        } else {
            f.debug_struct("RpcPriceFeedMetadataV2")
                .field("prev_publish_time", &self.prev_publish_time)
                .field("proof_available_time", &self.proof_available_time)
                .field("slot", &self.slot)
                .finish()
        }
    }
}