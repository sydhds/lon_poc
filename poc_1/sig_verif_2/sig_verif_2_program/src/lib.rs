use spel_framework::prelude::*;
use sig_verif_2_core::payload_verif;

#[account_type]
#[derive(Debug, Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct TokenState {
    pub amount: u64,
}

#[derive(Debug)]
struct GuardianSetInfo {
    pub addresses: [[u8; 20]; 13],
    pub expiration_time: u64,
}

#[lez_program]
mod sig_verif_2 {

    use super::*;
    use sig_verif_2_core::{GuardianSetInfo};

    /*
    const guardian_addresses: &[&[u8]] = &[
        &[0u8; 32],
        &[1u8; 32],
    ];
    */

    // Include GUARDIAN_SET_INFO
    include!(concat!(env!("OUT_DIR"), "/guardian_set_gen.rs"));

    #[instruction]
    pub fn initialize(
        #[account(init, pda = literal("token"))]
        mut token: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
    ) -> SpelResult {

        // println!("guardian_set_info: {:?}", GUARDIAN_SET_INFO);

        let state = TokenState {
            amount: 0,
        };
        let bytes = borsh::to_vec(&state).map_err(|e| SpelError::SerializationError {
            message: e.to_string(),
        })?;
        token.account.data = bytes.try_into().unwrap();

        Ok(SpelOutput::execute(vec![token, owner], vec![]))
    }

    #[instruction]
    pub fn mint(
        #[account(mut, pda = literal("token"))]
        mut token: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        amount: u64,
        // sig: Vec<u8>,
        payload: Vec<u8>,
    ) -> SpelResult {

        if !payload_verif(payload.as_slice(), &GUARDIAN_SET_INFO) {
            return SpelResult::Err(
                SpelError::Custom { code: 0, message: "Failed to parse/verify payload bytes".to_string() }
            );
        }

        // Update account
        let data: Vec<u8> = token.account.data.clone().into();
        let mut state: TokenState = borsh::from_slice(&data).map_err(|e| {
            SpelError::DeserializationError {
                account_index: 0,
                message: e.to_string(),
            }
        })?;

        state.amount = state.amount.checked_add(amount).ok_or(SpelError::Overflow {
            operation: "mint increment".to_string(),
        })?;

        let bytes = borsh::to_vec(&state).map_err(|e| SpelError::SerializationError {
            message: e.to_string(),
        })?;
        token.account.data = bytes.try_into().unwrap();

        Ok(SpelOutput::execute(vec![token, owner], vec![]))
    }

    #[instruction]
    pub fn bench_sig(
        #[account(mut, pda = literal("token"))]
        mut token: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        payload: Vec<u8>,
    ) -> SpelResult {

        use k256::ecdsa::{Signature, VerifyingKey, RecoveryId};
        use tiny_keccak::{Hasher, Keccak};

        #[derive(BorshDeserialize, Clone)]
        pub struct BenchmarkPayload {
            pub digest: [u8; 32],
            pub signature_bytes: [u8; 64],
            pub recovery_id: u8,
            pub expected_address: [u8; 20],
        }

        let payloads: Vec<BenchmarkPayload> = borsh::from_slice(&payload).map_err(|e| {
            SpelError::DeserializationError {
                account_index: 0,
                message: e.to_string(),
            }
        })?;

        for payload in payloads {

            // Recover public key
            let rec_id = RecoveryId::from_byte(payload.recovery_id).unwrap();
            let sig = Signature::from_slice(&payload.signature_bytes).unwrap();

            let recovered_key = VerifyingKey::recover_from_prehash(&payload.digest, &sig, rec_id)
                .expect("Recovery failed");

            // Get address
            let mut address = [0u8; 20];
            {
                let mut hasher = Keccak::v256();
                let encoded_point = recovered_key.to_encoded_point(false);
                hasher.update(&encoded_point.as_bytes()[1..]);

                let mut output = [0u8; 32];
                hasher.finalize(&mut output);
                address.copy_from_slice(&output[12..32]);
            }

            // Verify against expected address
            assert_eq!(address, payload.expected_address, "Address mismatch!");
        }

        Ok(SpelOutput::execute(vec![token, owner], vec![]))
    }
}
