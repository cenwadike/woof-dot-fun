# First Token CW20 contract (Copied from MANTRA)

This repository contains a sample implementation of a CW20 token contract for the CosmWasm smart contract platform. 
CW20 is a specification for fungible tokens similar to ERC20 on Ethereum, allowing for token creation, transfer, and
balance tracking within the Cosmos ecosystem.

## Features
- Minting: Allows the creation of new tokens.
- Burning: Tokens can be destroyed, reducing the total supply.
- Transfer: Users can send tokens to other addresses.
- Allowance: Provides the ability to approve other addresses to spend tokens on your behalf.
- Querying: Users can query balances, total supply, and allowances.


## Requirements
Rust and Cargo for building the contract.

## Getting Started

1. Clone the Repository

```bash
git clone https://github.com/MANTRA-Finance/first_token_cw20contract.git
cd first_token_cw20contract
```

2. Build the Contract

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.0
```

Or if you are on MacOS:

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer-arm64:0.16.0
```

3. Deploy the Contract

```bash
mantrachaind tx wasm store artifacts/first_token_cw20contract.wasm --from <wallet> --node https://rpc.hongbai.mantrachain.io:443 --chain-id mantra-hongbai-1 --gas-prices 0.35uom --gas auto --gas-adjustment 1.4 -y --output json
```
4. Instantiate the Contract

```bash
MSG='{
  "name": "MANTRACW20",
  "symbol": "MNTRA",
  "decimals": 6,
  "initial_balances": [
    {
      "address": "", // add your address here
      "amount": "10000000"
    }
  ]
}'

mantrachaind tx wasm instantiate <code_id> "$MSG" --from <wallet> --node https://rpc.hongbai.mantrachain.io:443 --chain-id mantra-hongbai-1 --label "MANTRAcw20" --no-admin --gas-prices 0.35uom --gas auto --gas-adjustment 1.4 -y --output json
```

## Refer to MANTRA Chain Docs for learning more

https://docs.mantrachain.io/
