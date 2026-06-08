use serde::{Deserialize, Serialize};
use spel_framework::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    Initialize,
    Mint { amount: u64, payload: Vec<u8> },
    BenchSig { payload: Vec<u8> },
}

pub fn main() {
    /*
    let (
        nssa_core::program::ProgramInput {
            self_program_id,
            caller_program_id,
            pre_states,
            instruction,
        },
        instruction_words,
    ) = nssa_core::program::read_nssa_inputs::<Instruction>();
    let pre_states_clone = pre_states.clone();

    println!("All good, instruction: {:?}", instruction);
    */

    {
        use k256::ecdsa::{signature::Signer, RecoveryId, Signature, SigningKey, VerifyingKey};
        use tiny_keccak::{Hasher, Keccak};

        let recover_bytes: u8 = 1;
        let digest_bytes = [23, 241, 140, 166, 133, 79, 66, 176, 246, 31, 166, 200, 55, 215, 196, 66, 48, 107, 100, 27, 192, 187, 40, 120, 190, 219, 57, 55, 184, 241, 175, 53];
        let signature_bytes = [166, 66, 83, 79, 81, 180, 100, 72, 169, 173, 217, 98, 128, 98, 16, 4, 89, 77, 194, 2, 39, 184, 213, 40, 226, 199, 151, 185, 59, 185, 76, 88, 46, 248, 148, 0, 223, 121, 213, 214, 63, 104, 203, 89, 43, 109, 110, 135, 119, 175, 59, 83, 202, 152, 67, 236, 43, 30, 151, 63, 7, 91, 51, 41];
        let expected_address = [209, 200, 127, 81, 54, 187, 32, 8, 236, 95, 83, 31, 8, 0, 75, 12, 203, 197, 129, 211];

        let rec_id = RecoveryId::from_byte(recover_bytes).unwrap();
        let sig = Signature::from_slice(&signature_bytes).unwrap();

        let recovered_key = VerifyingKey::recover_from_prehash(&digest_bytes, &sig, rec_id)
            .unwrap();

        let mut address = [0u8; 20];
        let mut hasher = Keccak::v256();
        hasher.update(&recovered_key.to_encoded_point(false).as_bytes()[1..]);
        let mut output = [0u8; 32];
        hasher.finalize(&mut output);
        address.copy_from_slice(&output[12..32]);

        assert_eq!(address, expected_address);
    }
}