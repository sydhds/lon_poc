/// Generate IDL JSON for the sig_verif_2 program.
///
/// Usage:
///   cargo run --bin generate_idl > sig_verif_2-idl.json

spel_framework::generate_idl!("../methods/guest/src/bin/sig_verif_2.rs");
