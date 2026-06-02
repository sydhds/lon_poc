mod state;
mod byte_utils;

use anyhow::{anyhow, Context};
use pythnet_sdk::accumulators::merkle::MerkleRoot;
use pythnet_sdk::hashers::keccak256_160::Keccak160;
use pythnet_sdk::messages::Message;
use pythnet_sdk::wire::{from_slice, PrefixedVec};
use serde::{Deserialize, Serialize};
// use sha3::{Digest, Keccak256};
use tiny_keccak::{Hasher, Keccak};
use k256::ecdsa::{VerifyingKey, Signature, RecoveryId};

/*
/// Example state struct — customize for your program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramState {
    pub initialized: bool,
    pub owner: [u8; 32],
}
*/
use pythnet_sdk::wire::v1::{AccumulatorUpdateData, MerklePriceUpdate, Proof, WormholeMessage, WormholePayload, PYTHNET_ACCUMULATOR_UPDATE_MAGIC};
use crate::byte_utils::ByteUtils;

pub fn payload_verif(payload: &[u8], guardian: &GuardianSetInfo) -> bool {

    if &payload[0..4] == PYTHNET_ACCUMULATOR_UPDATE_MAGIC {

    let update_data =
        AccumulatorUpdateData::try_from_slice(payload).unwrap();

        match update_data.proof {
            Proof::WormholeMerkle { vaa, updates } => {
                parse_and_validate_vaa(vaa.clone(), &guardian)
                    .unwrap();
                    // .context("Validate vaa")?;
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
    pub addresses:       [[u8; 20]; 13],
    pub expiration_time: u64, // Guardian set expiration time
}

/*
#[derive(Debug, Serialize, Deserialize)]
pub struct GuardianAddress {
    pub bytes: Vec<u8>,
}
*/

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

    // println!("root: {:?}", root);

    for update in updates {
        let message_vec = Vec::from(update.message);
        if !root.check(update.proof, &message_vec) {
            return Err(anyhow!("Invalid merkle proof"))?;
        }

        let msg = from_slice::<byteorder::BE, Message>(&message_vec)
            .map_err(|e| anyhow!(format!("Invalid accumulator message: {}", e)))?;

        match msg {
            Message::PriceFeedMessage(price_feed_message) => {
                // println!("Price feed message: {:?}", price_feed_message);
            }
            _ => { return Err(anyhow!("Invalid message type"))?; }
        }
    }

    Ok(())
}

fn parse_and_validate_vaa(vaa_: PrefixedVec<u16, u8>, guardian: &GuardianSetInfo) -> anyhow::Result<()> {

    let vaa = state::ParsedVAA::parse(vaa_.as_ref());
    if vaa.version != 1 {
        return Err(anyhow!("Invalid vaa version"));
    }

    /*
    // let guardian_set = guardians.get(&6).expect("Invalid guardian set");
    let now = Utc::now();
    // Note: guardians_set filtering (get the first non-expired guardian set)
    //       + this works because the last index has always an expiration_time of 0
    let guardian_set = guardians
        .into_iter()
        .filter(|(_, set_info)| set_info.expiration_time == 0 || set_info.expiration_time > now.timestamp_nanos_opt().unwrap() as u64)
        .last()
        .unwrap().1;
    */

    // println!("vaa len signers: {}", vaa.len_signers);

    // Let's calculate the digest that we are comparing against
    let mut pos =
        state::ParsedVAA::HEADER_LEN + (vaa.len_signers * state::ParsedVAA::SIGNATURE_LEN); //  SIGNATURE_LEN: usize = 66;

    let p1 = {
        // let mut hasher = Keccak256::default();
        let mut hasher = Keccak::v256(); // Sha3::v256();
        let mut output = [0u8; 32];
        hasher.update(&vaa_.as_ref()[pos..]);
        hasher.finalize(&mut output);
        output
    };

    let digest = {
        // let mut hasher = Keccak256::default();
        let mut hasher = Keccak::v256(); // Sha3::v256();
        let mut output = [0u8; 32];
        hasher.update(&p1);
        hasher.finalize(&mut output);
        output
    };

    // println!("Digest keccak256 done");

    let data = vaa_.as_ref().as_slice(); // .iter().as_slice();
    let mut last_index: i32 = -1;
    pos = state::ParsedVAA::HEADER_LEN;

    for _ in 0..vaa.len_signers {
        // which guardian signature is this?
        let index = data.get_u8(pos) as i32;
        // println!("index: {}", index);

        // We can't go backwards or use the same guardian over again
        if index <= last_index {
            return Err(anyhow!("Wrong guardian index order"));
        }
        last_index = index;
        pos += 1; // walk forward
        // Grab the whole signature
        let signature = &data[(pos)..(pos + state::ParsedVAA::SIG_DATA_LEN)]; // SIG_DATA_LEN: usize = 64;
        // println!("signature len: {:?}", signature.len());

        pos += state::ParsedVAA::SIG_DATA_LEN; // SIG_DATA_LEN: usize = 64;
        let recovery = data.get_u8(pos);

        // let v = env::ecrecover(&digest, signature, recovery, true).expect("cannot recover key");
        let v = {
            let recovery_id = RecoveryId::from_byte(recovery).expect("Invalid recovery ID");
            // println!("recovery id: {:?} - {:?}", recovery_id, recovery);
            let sig = Signature::from_slice(signature).expect("Invalid signature");
            // println!("signature: {:?}", sig);
            let recovered_public_key = VerifyingKey::recover_from_prehash(&digest, &sig, recovery_id)
                .expect("Failed to recover key");
            recovered_public_key
        };
        // let k = &env::keccak256(&v)[12..32];
        let mut address_k = [0u8; 20];
        let k_ = {
            // let mut hasher = Keccak256::default();
            let mut hasher = Keccak::v256(); // Sha3::v256();
            let mut output = [0u8; 32];

            // to_sec1_bytes returns the uncompressed public key (65 bytes long)
            // with byte 0 (0x04): prefix for uncompressed key
            // ecrecover return a 64 bytes array (so without the prefix key)
            // by using to_encoded_point(false) we ensure we got an uncompressed public key
            // hasher.update(&v.to_sec1_bytes());
            let encoded_point = v.to_encoded_point(false);
            hasher.update(&encoded_point.as_bytes()[1..]);

            hasher.finalize(&mut output);
            // output.to_vec()
            address_k.copy_from_slice(&output[12..32]);
        };
        // let k = &k_.as_slice()[12..32];

        // println!("guardian index: {}", index);
        let key = guardian.addresses.get(index as usize).unwrap();
        
        if address_k != *key {
            println!("Sgnature_error: {:?} != {:?}",
                     &address_k,
                     &key,
            );
            return Err(anyhow!("Guardian signature error"));
        } else {
            println!("== Guardian signature OK ==");
        }
        
        /*
        if k != key.bytes {

            println!("Sgnature_error: {:?} != {:?}",
                     &k,
                     &key.bytes,
            );


            return Err(anyhow!("Guardian signature error"));
        } else {
            println!("== Guardian signature OK ==");
        }
        */
        
        pos += 1;
    }

    Ok(())
}

