## Pyth fetch

Fetch Pyth (pyth network) payload (aka Price update) for a given price feed.

## Fetch

* `cargo run -- fetch`
  * `target/debug/pyth_fetch fetch`

Note: after a fetch, the price feed is cached in `price_latest.json`

## Read

Note: you need an initial call to `fetch` (generate a `price_latest.json` file)

* `cargo run -- read`
  * `target/debug/pyth_fetch read`


