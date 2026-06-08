use k256::ecdsa::{SigningKey, signature::Signer};
use tiny_keccak::{Hasher, Keccak};
use rand::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use k256::elliptic_curve::rand_core::OsRng;

#[derive(Debug, Clone, BorshSerialize)]
pub struct BenchmarkPayload {
    pub digest: [u8; 32],
    pub signature_bytes: [u8; 64],
    pub recovery_id: u8,
    pub expected_address: [u8; 20],
}

fn generate_mock_payloads(count: usize) -> Vec<BenchmarkPayload> {
    let mut payloads = Vec::new();

    for _ in 0..count {
        // Generate a random private key
        let signing_key = SigningKey::random(&mut OsRng);
        
        // Mock a 32-byte Pyth digest
        let digest: [u8; 32] = rand::random();

        // Sign the digest (this gives us the sig + recovery ID)
        let (signature, recovery_id) = signing_key.sign_prehash_recoverable(&digest).unwrap();

        // Calculate the expected Ethereum address for the verifier to check against
        let verifying_key = signing_key.verifying_key();
        let mut expected_address = [0u8; 20];
        let mut hasher = Keccak::v256();
        hasher.update(&verifying_key.to_encoded_point(false).as_bytes()[1..]);
        let mut output = [0u8; 32];
        hasher.finalize(&mut output);
        expected_address.copy_from_slice(&output[12..32]);

        payloads.push(BenchmarkPayload {
            digest,
            signature_bytes: signature.to_bytes().into(),
            recovery_id: recovery_id.to_byte(),
            expected_address,
        });

        // println!("Generated payload: {:?}", payloads.last().unwrap());
    }

    payloads
}

fn main() {

    let count = std::env::args().nth(1).unwrap_or("5".to_string()).parse::<usize>().unwrap();

    println!("Generating a payload with {} signatures to verify...", count);

    let payloads = generate_mock_payloads(count);
    println!("Generated {} payloads", payloads.len());

    let payload_bytes_ = borsh::to_vec(&payloads).unwrap();
    let payload_bytes = payload_bytes_
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(",");
    println!("payload bytes: {:?}", payload_bytes);

}