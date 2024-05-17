add_denom_msg=$(jq -n \
  '{
    "sudo": {
      "add_new_denom": {
        "denom": "factory/stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le/art3mix",
        "limits": {
          "min": "1000000",
          "max": "10000000",
          "bank": "100000000"
        }
      }
    }
  }')

starsd tx wasm execute stars16fdgsjm60yrknend2kwl0gw90tstyzvvgr5dre0mkfuktl83q7ascv28xj "$add_denom_msg" --amount 100000000factory/stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le/art3mix --from main --gas-prices 1ustars --gas-adjustment 1.4 --gas auto -b block -y
