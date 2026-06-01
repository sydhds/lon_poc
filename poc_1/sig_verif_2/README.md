# sig_verif_2

LON POC pyth signature & merkle root verification

## Build

### Build setup

Generate guardian set info json:
* `cd ../guardians_extract && cargo run -- fetch && cp -v ../guardian_extract/guardian.json methods/guest/`

Easy option (build contract, generate idl & deploy contract): 
* `./build.sh`

## Interact with the contract

* initialize the contract:
  * `spel initialize --owner <OWNER_ADDRESS>`
    * example: `spel initialize --owner 7e8dDMEsTj1RmJ6BZ3DXnimxWQMNQDpYkGbMsAfk8BKs`
  * mint token:
    * fetch pyth to get payload:
      * `cd ../pyth_extract && cargo run -- fetch`
        * The payload is ready to past (check for `pyth_payload (ready_for_spel)`)
    * `spel mint --owner 7e8dDMEsTj1RmJ6BZ3DXnimxWQMNQDpYkGbMsAfk8BKs --amount 15 --payload "{PAYLOAD}"`
