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
}
