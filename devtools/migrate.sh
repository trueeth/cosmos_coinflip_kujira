migrate_msg=$( jq -n \
  '{
    "from_v07": {
      "nft_pool_max": 5,
      "streak_nft_winning_amount": 3,
      "streak_rewards": [{
        "streak": 1,
        "reward": "500000"},
        {"streak": 2,
        "reward": "1000000"},
        {"streak": 3,
        "reward": "5000000"}],
      "allowed_to_send_nft": ["stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le"]
    }
  }')

starsd tx wasm migrate stars16fdgsjm60yrknend2kwl0gw90tstyzvvgr5dre0mkfuktl83q7ascv28xj 3420 "$migrate_msg" --from main --gas-prices 1ustars --gas-adjustment 1.4 --gas auto -b block -y
