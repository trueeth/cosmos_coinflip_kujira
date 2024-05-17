read -r -d '' desc << EOM
## Summary - Store WASM Code

Cosmos Coinflip is a coinflip game for Cosmos IBC enabled tokens. We are launching first on Stargaze where users are able to flip head or tails on a set amount of stars to double or nothing. This all comes with an exclusive Pixel art NFT collection on stargaze that has some benefits of their own!

The stargaze team has reviewed the contract and provided feedback which was addressed.

Github Repo for review: https://github.com/Cosmos-Coin-Flip/Cosmos-Coin-Flip

Testnet Website: https://testnet.cosmoscoinflip.com

Twitter: https://twitter.com/CosmosCoinFlip

Discord: https://discord.gg/qaNZCpTfhE

## Compile Instructions

Source code: https://github.com/Cosmos-Coin-Flip/Cosmos-Coin-Flip/tree/v0.7.2

docker run --rm -v "$(pwd)":/code --platform linux/amd64 \
	--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
	--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
	cosmwasm/workspace-optimizer:0.12.13

Or clone the repo and run: " ./devtools/optimizer.sh "

This results in the following SHA256 checksum:

86077c3caf35eb66d58b6380f69efaaf1a979c09d3b08af118ad398c160fdec9

## Verify On-chain Contract

starsd q gov proposal $id --output json \\
| jq -r '.content.wasm_byte_code' \\
| base64 -d \\
| gzip -dc \\
| sha256sum

## Verify Local Contract
sha256sum artifacts/coin_flip.wasm
EOM

title="Upload Cosmos Coin Flip game smart contract"

starsd tx gov submit-proposal wasm-store ../artifacts/coin_flip.wasm \
--title "$title" \
--description "$desc" \
--run-as "stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le" \
--builder "cosmwasm/workspace-optimizer:0.12.13" \
--code-hash "86077c3caf35eb66d58b6380f69efaaf1a979c09d3b08af118ad398c160fdec9" \
--code-source-url "https://github.com/Cosmos-Coin-Flip/Cosmos-Coin-Flip/tree/v0.7.2" \
--instantiate-anyof-addresses "stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le" \
--deposit 20000000000ustars \
--chain-id "stargaze-1" \
--node "https://rpc.stargaze-apis.com:443" \
--from main \
--gas-prices 0ustars \
--gas-adjustment 1.3 \
--gas auto \
-b block -y
