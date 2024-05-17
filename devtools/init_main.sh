code_id=46
init_msg=$( jq -n \
  '{
    admin: "",
    denoms: ["ustars"],
    fees: {
      flip_bps: 300,
      holders_bps: 7000,
      reserve_bps: 1500,
      team_bps: 1500,
    },
    flips_per_block_limit: 10,
    bank_limit: "30000000000",
    min_bet_limit: "50000000",
    max_bet_limit: "250000000",
    wallets: {
      reserve: "stars1ngygw5sw0dfchq9ffm5p2hf2z7ndq4lu9dh6fm",
      team: "stars12d0k89lxp224xke3fe4mzfxpnxjshwqcvtxnd8",
    },
  }')

starsd tx wasm instantiate $code_id "$init_msg" --label "coin_flip" \
--admin stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le \
--gas-prices 0.025ustars --gas auto --gas-adjustment 1.9 --from main \
--node https://stargaze-rpc.polkachu.com:443 --chain-id stargaze-1 -b block -y
