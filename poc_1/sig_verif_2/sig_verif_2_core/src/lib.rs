mod byte_utils;
mod state;

use anyhow::{anyhow, Context};
use pythnet_sdk::accumulators::merkle::MerkleRoot;
use pythnet_sdk::hashers::keccak256_160::Keccak160;
use pythnet_sdk::messages::Message;
use pythnet_sdk::wire::{from_slice, PrefixedVec};
use serde::{Deserialize, Serialize};
// use sha3::{Digest, Keccak256};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use tiny_keccak::{Hasher, Keccak};

/*
/// Example state struct — customize for your program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramState {
    pub initialized: bool,
    pub owner: [u8; 32],
}
*/
use crate::byte_utils::ByteUtils;
use pythnet_sdk::wire::v1::{
    AccumulatorUpdateData, MerklePriceUpdate, Proof, WormholeMessage, WormholePayload,
    PYTHNET_ACCUMULATOR_UPDATE_MAGIC,
};

pub fn payload_verif(payload: &[u8], guardian: &GuardianSetInfo) -> bool {
    if &payload[0..4] == PYTHNET_ACCUMULATOR_UPDATE_MAGIC {
        let update_data = AccumulatorUpdateData::try_from_slice(payload).unwrap();

        match update_data.proof {
            Proof::WormholeMerkle { vaa, updates } => {
                parse_and_validate_vaa(vaa.clone(), &guardian).unwrap();
                // .context("Validate vaa")?;
                validate_merkle_proof(vaa, updates).unwrap()
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
    pub addresses: [[u8; 20]; 19],
    pub expiration_time: u64, // Guardian set expiration time
}

/*
#[derive(Debug, Serialize, Deserialize)]
pub struct GuardianAddress {
    pub bytes: Vec<u8>,
}
*/

//

fn validate_merkle_proof(
    vaa_: PrefixedVec<u16, u8>,
    updates: Vec<MerklePriceUpdate>,
) -> anyhow::Result<()> {
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
            _ => {
                return Err(anyhow!("Invalid message type"))?;
            }
        }
    }

    Ok(())
}

fn parse_and_validate_vaa(
    vaa_: PrefixedVec<u16, u8>,
    guardian: &GuardianSetInfo,
) -> anyhow::Result<()> {
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
            let recovered_public_key =
                VerifyingKey::recover_from_prehash(&digest, &sig, recovery_id)
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

        println!("guardian index: {}", index);
        let key = guardian.addresses.get(index as usize).unwrap();

        if address_k != *key {
            println!("Sgnature_error: {:?} != {:?}", &address_k, &key,);
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

#[cfg(test)]
mod tests {
    use super::*;
    use risc0_zkvm::{default_prover, ExecutorEnv};

    const PAYLOAD: &str = r#"
        {
  "binary": {
    "data": [
      "504e41550100000003b801000000060d005e3471f24af1ea3ae218544254a9d4a2c4525f38958bdd5a11391dd4e6cde0657e62c1c595a7ac955adb70815422e37c961cfba7ca929b416469ec775ed9d154000120aa5c0a329db823063b0041c7c3313563a6ae00d2dec7c549abde9f956330053b16a25912323553461dbf01a34139e1454b033347b1a003ba53528518b28bf80102a06f862d30a4413d15e8ea5cb8b27f318a27fd1a5f85a0eefdde924304aaa9454b2142fea318ed1241c056f7f0089439bdc349df281a17bd3a83d94e9e7647260104b059cac44b9690b5c0c8ccc1e6119f08984cdf9e7c0c6868f99ca8104e7db36d6dfacdbbcea9c16e3a157ac1ef5c65dc4ba6eb1ceb3db2664a435045419f8d7f000584d637e6ad47ad90cb2e5c528498085f711b240d727cd866af185a967dddcbdd559b7dd9939698675daa9a38e1c93986220abd3fe28191af6e24fbd30d3b52c30106fbc9bb018bbcd42dc605a5363be5b6167d651c7952e235e3a901cbbf330c897b56d83d43dc9c9613eafb9330c590b62a4d03f6068d98de2fa47576aafb8bd92200076878e4df73543a4ab33151b0ff263fc88f2f45ca6bdcc8b103ba26a61331f4f4040cab39ea9103c5526c84e2d108cedf930f1e0aa6c94086fbec9cf4ebd373ab01085408a27a9dd18c1ad11af254fe33e248401ec7fd19173d7c8a1ffd9113b09cb4543ca28706010fb595ac53b1e5e304fb87652202016d2bf66f6a53a8a1b2fc44000a65bcf1e0071fa7ba1fb467a51fa8c50e6af4db69d517619e21683f220b3ee87228a8a5c65ef4fd67dc6ff615fd6638d9a94acef6ff7ca9dc622ee393b5f7e77a010b030b532bec470ed5cb0c985955bdb690fb3d98be835bbece1abfda93033be0f61931e7ce10b1c7bbcc2befb43d77051afdde5114d30055816e9b637aed3d3a86000dc85b824c1df462d265e5a5acb1970f331fa366f268504cd4a36e046ca3282663666808fee2ab438de8cfe38c2474dabde0e5c57a1d55829f0a65041c5ca756ec0110a490857424cc457f770fd5efcc05ddaa1fe265771092b875cb8da30f08df37b409fa11025cd6134eeb750cf52f1659cdd3ae5a502a4460981dfae9e67c0172360012e43cd51e9a88ad2d03f2ea4146140094b7b495c2c076c4f582c2ac134a6362f112e0b27ccebaad74f05bef691fefeb54b398207ef1f9a19b44f844e8c8760e57006a1d43f100000000001ae101faedac5851e32b9b23b5f9411a8c2bac4aae3ed4dd7b811dd1a72ea4aa71000000000c7a6e1b0141555756000000000011881ba100002710cf8d1a143ebcbd2f54a46ba49117cf1e6a5f1aed01005500ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace0000002e05f5f2200000000009da9ea3fffffff8000000006a1d43f1000000006a1d43f10000002e1de07568000000000ae162ff0d7eb9861aca3596d9916f0c294a04a8454cf40b707a0e848ff7dc3008f70b5136fde69e5c2eb4c08eb657f0958a7f77b5bb648864d8c6ed34354a221b0ca489f8646f7d86a0fa5df6a5f0510bc4992e3de87204711004ebfddf77aa845012ef65df4d64b6258d38cd39d9e8e7549238233c9d78341ec543d2dcebffedb65b08655102b149fd7e5e93ea4f5bfa5b5ecca5b5e43a554f09352f961bf3f3426cfc9e5ab8f9f1d684ce4cb543198b1f7e9cf3f0eb28ecffa15d886dfb16c89422794511a8e92a6fbbaf52f32793b0b7f3c61532f49d7c0aa42be46de7944bb6bc93076392c63b0b4530553e82bb5092deaf3f4a9356d60a4a3461ba95ba172a09ccecc14eb8e4"
    ],
    "encoding": "hex"
  },
  "parsed": [
    {
      "ema_price": {
        "conf": "182543103",
        "expo": -8,
        "price": "198069745000",
        "publish_time": 1780302833
      },
      "id": "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
      "metadata": {
        "prev_publish_time": 1780302833,
        "proof_available_time": 1780302835,
        "slot": 294132641
      },
      "price": {
        "conf": "165322403",
        "expo": -8,
        "price": "197668500000",
        "publish_time": 1780302833
      }
    }
  ]
}
    "#;

    const GUARDIAN_SET_INFO: GuardianSetInfo = GuardianSetInfo {
        addresses: [
            [
                88, 147, 181, 167, 108, 63, 115, 150, 69, 100, 136, 133, 189, 204, 192, 108, 215,
                10, 60, 211,
            ],
            [
                255, 108, 185, 82, 88, 155, 222, 134, 44, 37, 239, 67, 146, 19, 47, 185, 212, 164,
                33, 87,
            ],
            [
                17, 77, 232, 70, 1, 147, 189, 243, 162, 252, 248, 31, 134, 160, 151, 101, 244, 118,
                47, 209,
            ],
            [
                16, 122, 0, 134, 179, 45, 122, 9, 119, 146, 106, 32, 81, 49, 216, 115, 29, 57, 203,
                235,
            ],
            [
                140, 130, 178, 253, 130, 250, 237, 39, 17, 213, 154, 240, 242, 73, 157, 22, 231,
                38, 246, 178,
            ],
            [
                66, 87, 155, 255, 188, 244, 39, 110, 41, 10, 184, 228, 193, 98, 189, 64, 82, 185,
                121, 112,
            ],
            [
                147, 143, 16, 74, 235, 85, 129, 41, 50, 22, 206, 151, 215, 113, 224, 203, 114, 18,
                33, 177,
            ],
            [
                24, 228, 22, 116, 204, 242, 99, 41, 205, 17, 20, 6, 193, 208, 92, 108, 128, 178,
                62, 220,
            ],
            [
                157, 22, 135, 1, 96, 231, 3, 50, 77, 5, 124, 51, 97, 195, 76, 91, 239, 186, 44, 52,
            ],
            [
                0, 10, 192, 7, 103, 39, 179, 95, 190, 162, 218, 194, 143, 238, 92, 203, 15, 234,
                118, 142,
            ],
            [
                175, 69, 206, 209, 54, 185, 217, 226, 73, 3, 70, 74, 232, 137, 245, 200, 167, 35,
                252, 20,
            ],
            [
                249, 49, 36, 183, 199, 56, 132, 60, 187, 137, 232, 100, 200, 98, 195, 140, 221,
                204, 207, 149,
            ],
            [
                210, 204, 55, 164, 220, 3, 106, 141, 35, 43, 72, 246, 44, 221, 71, 49, 65, 47, 72,
                144,
            ],
            [
                218, 121, 143, 104, 150, 163, 51, 31, 100, 180, 140, 18, 209, 213, 127, 217, 203,
                231, 8, 17,
            ],
            [
                209, 246, 78, 38, 35, 136, 17, 222, 85, 83, 196, 15, 100, 175, 65, 238, 27, 96, 87,
                204,
            ],
            [
                63, 133, 26, 213, 134, 164, 124, 239, 141, 4, 116, 143, 51, 171, 13, 113, 57, 95,
                6, 180,
            ],
            [
                23, 142, 33, 173, 46, 119, 174, 6, 113, 21, 73, 207, 187, 31, 156, 122, 157, 128,
                150, 232,
            ],
            [
                120, 153, 206, 171, 29, 201, 97, 218, 233, 222, 253, 183, 164, 245, 33, 38, 154,
                84, 72, 252,
            ],
            [
                111, 190, 188, 137, 143, 64, 62, 71, 115, 233, 95, 235, 21, 232, 12, 154, 153, 200,
                52, 141,
            ],
        ],
        expiration_time: 0,
    };

    #[derive(Debug, Serialize, Deserialize)]
    struct PriceUpdate {
        binary: BinaryUpdate,
        // Skip this (not needed)
        // parsed: Vec<ParsedPriceUpdate>
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct BinaryUpdate {
        data: Vec<String>,
        encoding: String,
    }

    #[test]
    fn test_payload_verif() -> anyhow::Result<()> {

        // Payload
        let price_update: PriceUpdate = serde_json::from_str(PAYLOAD).unwrap();
        let payload_bytes = &*hex::decode(price_update.binary.data[0].as_str())?;
        // let result = payload_bytes
        //     .iter()
        //     .map(|b| b.to_string())
        //     .collect::<Vec<String>>()
        //     .join(",");
        // println!("pyth payload (ready for spel): {}", result);

        assert!(payload_verif(payload_bytes, &GUARDIAN_SET_INFO));

        Ok(())
    }

    #[test]
    fn test_vm_payload_verif() {
        // Payload
        let price_update: PriceUpdate = serde_json::from_str(PAYLOAD).unwrap();
        let payload_bytes = &*hex::decode(price_update.binary.data[0].as_str())?;

        // 1. Prepare your mock VAA payload data
        // let mock_vaa_payload: Vec<u8> = vec![/* inject mock bytes here */];

        // 2. Build the Executor Environment
        let env = ExecutorEnv::builder()
            .write(&mock_vaa_payload).unwrap()
            // Bump the limit specifically for the test so it doesn't panic
            .session_limit(Some(100_000_000))
            .build()
            .unwrap();

        // 3. Get the default prover
        let prover = default_prover();

        // 4. EXECUTE the code (Bypasses the slow STARK prover)
        // We use `.execute()` instead of `.prove()` for lightning-fast testing
        let session_info = prover.execute(env, YOUR_GUEST_METHOD_ELF)
            .expect("Guest execution panicked or failed!");

        // 5. (Optional) Read the journal if your guest commits data back
        // let result: bool = session_info.journal.decode().unwrap();
        // assert!(result);

        println!("Test passed! Execution finished successfully.");
    }
}
