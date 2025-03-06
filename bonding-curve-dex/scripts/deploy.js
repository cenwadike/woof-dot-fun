import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import { readFileSync } from "fs";

import dotenv from "dotenv"

dotenv.config()

const rpcEndpoint = "https://rpc-palvus.pion-1.ntrn.tech";
const mnemonic = process.env.MNEMONIC;
const wasmFilePath = "../artifacts/bonding_curve_dex.wasm";

async function main() {
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: "neutron",
  });

  const [firstAccount] = await wallet.getAccounts();

  const client = await SigningCosmWasmClient.connectWithSigner(
    rpcEndpoint,
    wallet,
    {
      gasPrice: GasPrice.fromString("0.025untrn"),
    }
  );

  // const wasmCode = readFileSync(wasmFilePath);
  // const uploadReceipt = await client.upload(firstAccount.address, wasmCode, "auto");
  // console.log("Upload successful, code ID:", uploadReceipt.codeId);

  const initMsg = {
    token_factory: "neutron15e8pmchvjyx3uecc9zqt82c90vxghgxcu6chexwuwggp4yh5tdkq36mc5w",
    fee_collector: JSON.stringify(firstAccount.address),
    quote_token_total_supply: JSON.stringify(100_000_000_000 * 10**9),
    bonding_curve_supply: JSON.stringify(80_000_000_000 * 10**9),
    lp_supply: JSON.stringify(20_000_000_000 * 10**9),
    maker_fee: JSON.stringify(0.01),
    taker_fee: JSON.stringify(0.02),
    secondary_amm_address: "osmosis-escrow",
    base_token_denom: "untrn"
  };

  const instantiateReceipt = await client.instantiate(
    firstAccount.address, 
    11034, // uploadReceipt.codeId, 
    initMsg, 
    "Woof.fun Test", 
    "auto"
  );
  console.log("Contract instantiated at:", instantiateReceipt.contractAddress);
    // Upload successful, code ID: 11034
    // Contract instantiated at: neutron1g8d23dxx5haeg0rxt83apptmyl004rh4m7dvtmnzarmlgde29jcqc593ul
}

main().catch(console.error);
