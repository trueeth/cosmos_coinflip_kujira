
# starsd query wasm contract-state smart stars1vy9z85pp3zymdz6vw2gyqg27gh264t7gas6ufg9gzx72pxz9s6nq34y5c9 \
# '{"dry_distribution":{}}' \
# --node https://stargaze-rpc.polkachu.com:443 --chain-id stargaze-1

starsd tx wasm execute stars1vy9z85pp3zymdz6vw2gyqg27gh264t7gas6ufg9gzx72pxz9s6nq34y5c9 \
'{"sudo":{"distribute": {}}}' --gas-prices 1ustars --gas auto --gas-adjustment 1.4 --from main \
--node https://stargaze-rpc.polkachu.com:443 --chain-id stargaze-1 -b block -y
