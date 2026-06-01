use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

// Duplicate your struct definition here so the build script can parse it
#[derive(Deserialize, Debug)]
struct GuardianSetInfo_ {
    pub addresses: Vec<GuardianAddress_>,
    pub expiration_time: u64,
}

#[derive(Debug, Deserialize)]
pub struct GuardianAddress_ {
    pub bytes: Vec<u8>,
}

/*
#[derive(Debug)]
struct GuardianSetInfo {
    pub addresses: [[u8; 20]; 13],
    pub expiration_time: u64,
}
*/

fn main() {
    // Tell Cargo to recompile if the JSON file changes
    println!("cargo:rerun-if-changed=guardian.json");

    let json_content = fs::read_to_string("guardian.json").unwrap();
    let guardian: GuardianSetInfo_ = serde_json::from_str(&json_content).unwrap();

    println!("guardian: {:?}", guardian);
    println!("guardian addresses len: {:?}", guardian.addresses.len());

    // 2. Format it into standard Rust code
    let generated_code = format!(r#"

        const addr_1: [u8; 20] = {:?};
        const addr_2: [u8; 20] = {:?};
        const addr_3: [u8; 20] = {:?};
        const addr_4: [u8; 20] = {:?};
        const addr_5: [u8; 20] = {:?};
        const addr_6: [u8; 20] = {:?};
        const addr_7: [u8; 20] = {:?};
        const addr_8: [u8; 20] = {:?};
        const addr_9: [u8; 20] = {:?};
        const addr_10: [u8; 20] = {:?};
        const addr_11: [u8; 20] = {:?};
        const addr_12: [u8; 20] = {:?};
        const addr_13: [u8; 20] = {:?};
        pub const GUARDIAN_SET_INFO: GuardianSetInfo = GuardianSetInfo {{ expiration_time: {}, addresses: [addr_1, addr_2, addr_3, addr_4, addr_5, addr_6, addr_7, addr_8, addr_9, addr_10, addr_11, addr_12, addr_13] }};
    "#,
                                 guardian.addresses[0].bytes,
                                 guardian.addresses[1].bytes,
                                 guardian.addresses[2].bytes,
                                 guardian.addresses[3].bytes,
                                 guardian.addresses[4].bytes,
                                 guardian.addresses[5].bytes,
                                 guardian.addresses[6].bytes,
                                 guardian.addresses[7].bytes,
                                 guardian.addresses[8].bytes,
                                 guardian.addresses[9].bytes,
                                 guardian.addresses[10].bytes,
                                 guardian.addresses[11].bytes,
                                 guardian.addresses[12].bytes,
                                 guardian.expiration_time,
    );

    // 3. Write the Rust code to the build output directory
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("guardian_set_gen.rs");
    println!("Generated in: {}", dest_path.display());
    fs::write(&dest_path, generated_code).unwrap();
}