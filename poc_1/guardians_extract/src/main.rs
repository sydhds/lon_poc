use std::collections::HashMap;
use clap::Parser;
use serde::Serialize;
use chrono::{DateTime, Utc};
// Near crates
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_primitives::{
    borsh::BorshDeserialize,
    types::Finality,
    views::QueryRequest
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    /*
    // Near mainnet
    let client = JsonRpcClient::connect("https://rpc.mainnet.near.org");
    // let client = JsonRpcClient::connect("https://rpc.testnet.near.org");

    let contract_id = "contract.wormhole_crypto.near";
    // let contract_id = "wormhole.wormhole.testnet";
    */

    let args = Args::parse();
    let client = JsonRpcClient::connect(args.rpc_url);

    println!("Query'ing contract state (id: {})...", args.contract_id);

    // Query state with prefix: "gs"
    // See field init. in wormhole contract: guardians: LookupMap::new(b"gs".to_vec()),
    let request = methods::query::RpcQueryRequest {
        block_reference: Finality::Final.into(),
        request: QueryRequest::ViewState {
            account_id: args.contract_id.parse()?,
            prefix: b"gs".to_vec().into(),
            include_proof: false,
        },
    };

    let response = client.call(request).await?;
    if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewState(state) = response.kind {

        if state.values.is_empty() {
            println!("No guardian sets found. Check the contract ID or prefix.");
            return Ok(());
        }

        let mut guardians: HashMap<u32, GuardianSetInfo> = Default::default();

        for item in state.values {

            // println!("item key: {:?}", item.key);

            match GuardianSetInfo::try_from_slice(&item.value) {
                Ok(guardian_set) => {
                    println!("\n## Guardian Set");
                    println!("* Expiration Time: {}", guardian_set.expiration_time);
                    // let dt = DateTime::from_timestamp(guardian_set.expiration_time as i64, 0).expect("invalid timestamp");
                    let dt = DateTime::from_timestamp_nanos(guardian_set.expiration_time as i64);
                    println!("* Expiration Time: {}", dt);

                    for (i, address) in guardian_set.addresses.iter().enumerate() {
                        let hex_address = hex::encode(&address.bytes);
                        // println!("* Guardian {}: 0x{} ({:?})", i, hex_address, address.bytes);
                    }

                    // item.key == "gs" + u32 index
                    let index: u32 = BorshDeserialize::deserialize(&mut &item.key[2..])?;
                    println!("* item key: {:?} (index: {})", item.key, index);
                    guardians.insert(index, guardian_set);
                },
                Err(e) => {
                    println!("Found a 'gs' key but failed to decode it: {}", e);
                }
            }
        }

        println!("Writing full guardians set to file: {}...", args.output_json);
        let fs = std::fs::File::create(args.output_json)?;
        serde_json::to_writer_pretty(fs, &guardians)?;

        println!("Writing filtered guardians set to file: {}...", args.output_valid);
        let fs = std::fs::File::create(args.output_valid)?;
        let now = Utc::now();

        let guardian_set_info_valid = guardians
            .into_iter()
            .filter_map(|(_, gs)| {
                let expiration_date = DateTime::from_timestamp_nanos(gs.expiration_time as i64);
                if gs.expiration_time == 0 || expiration_date > now {
                    Some(gs)
                } else { None }
            })
            .next()
            .unwrap();
        println!("Guardian set: {:?}", guardian_set_info_valid);
        serde_json::to_writer_pretty(fs, &guardian_set_info_valid)?;
    }

    Ok(())
}

// From wormhole contract
// https://github.com/pyth-network/wormhole/blob/main/near/contracts/wormhole/src/lib.rs
#[derive(BorshDeserialize, Debug, Serialize)]
pub struct GuardianAddress {
    pub bytes: Vec<u8>,
}

#[derive(BorshDeserialize, Debug, Serialize)]
pub struct GuardianSetInfo {
    pub addresses: Vec<GuardianAddress>,
    pub expiration_time: u64,
}

// Cli Args

#[derive(Parser, Debug)]
#[command(name = "near_sc_extract")]
#[command(version = "1.0")]
#[command(about = "Dev utilities", long_about = None)]
pub struct Args {
    #[arg(
        short,
        long = "rpc-url",
        default_value = "https://rpc.mainnet.near.org",
    )]
    pub rpc_url: String,

    #[arg(short, long = "contract-id", default_value = "contract.wormhole_crypto.near")]
    pub contract_id: String,

    #[arg(short, long = "output", default_value = "guardians.json")]
    pub output_json: String,

    #[arg(long = "output_valid", default_value = "guardian.json")]
    pub output_valid: String,
}
