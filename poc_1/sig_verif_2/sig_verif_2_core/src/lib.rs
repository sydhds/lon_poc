mod state;
mod byte_utils;

use anyhow::anyhow;
use pythnet_sdk::accumulators::merkle::MerkleRoot;
use pythnet_sdk::hashers::keccak256_160::Keccak160;
use pythnet_sdk::messages::Message;
use pythnet_sdk::wire::{from_slice, PrefixedVec};
use serde::{Deserialize, Serialize};

/*
/// Example state struct — customize for your program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramState {
    pub initialized: bool,
    pub owner: [u8; 32],
}
*/
use pythnet_sdk::wire::v1::{AccumulatorUpdateData, MerklePriceUpdate, Proof, WormholeMessage, WormholePayload, PYTHNET_ACCUMULATOR_UPDATE_MAGIC};

pub fn payload_verif(payload: &[u8]) -> bool {

    if &payload[0..4] == PYTHNET_ACCUMULATOR_UPDATE_MAGIC {

    let update_data =
        AccumulatorUpdateData::try_from_slice(payload).unwrap();

        match update_data.proof {
            Proof::WormholeMerkle { vaa, updates } => {
                // parse_and_validate_vaa(vaa.clone(), &guardian).context("Validate vaa")?;
                validate_merkle_proof(vaa, updates)
                    .unwrap()
                    // .context("Validate merkle proof")?;
            }
        }

        return true;
    } else {
        return false;
    }
}

// From Wormhole SC contract

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardianSetInfo {
    pub addresses:       Vec<GuardianAddress>,
    pub expiration_time: u64, // Guardian set expiration time
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardianAddress {
    pub bytes: Vec<u8>,
}

//

fn validate_merkle_proof(vaa_: PrefixedVec<u16, u8>, updates: Vec<MerklePriceUpdate>) -> anyhow::Result<()> {

    let vaa = state::ParsedVAA::parse(vaa_.as_ref());
    if vaa.version != 1 {
        return Err(anyhow!("Invalid vaa version"));
    }

    let message = WormholeMessage::try_from_bytes(vaa.payload)?;
    let root: MerkleRoot<Keccak160> = MerkleRoot::new(match message.payload {
        WormholePayload::Merkle(merkle_root) => merkle_root.root,
    });

    println!("root: {:?}", root);

    for update in updates {
        let message_vec = Vec::from(update.message);
        if !root.check(update.proof, &message_vec) {
            return Err(anyhow!("Invalid merkle proof"))?;
        }

        let msg = from_slice::<byteorder::BE, Message>(&message_vec)
            .map_err(|e| anyhow!(format!("Invalid accumulator message: {}", e)))?;

        match msg {
            Message::PriceFeedMessage(price_feed_message) => {
                println!("Price feed message: {:?}", price_feed_message);
            }
            _ => { return Err(anyhow!("Invalid message type"))?; }
        }
    }

    Ok(())
}