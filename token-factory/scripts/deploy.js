import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import { readFileSync } from "fs";

import dotenv from "dotenv"

dotenv.config()

const rpcEndpoint = "https://rpc-palvus.pion-1.ntrn.tech";
const mnemonic = process.env.MNEMONIC;
const wasmFilePath = "../artifacts/token_factory.wasm";

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

  const wasmCode = readFileSync(wasmFilePath);
  const uploadReceipt = await client.upload(firstAccount.address, wasmCode, "auto");
  console.log("Upload successful, code ID:", uploadReceipt.codeId);

  const initMsg = {
    token_code_id: 11031,
    token_code_hash: "5fee983db91565497b13238c30935d39ff15bd7fe12bb6c4946a5d18b41fa58e"
  };

  const instantiateReceipt = await client.instantiate(
    firstAccount.address, 
    uploadReceipt.codeId, 
    initMsg, 
    "Woof.fun Test", 
    "auto"
  );
  console.log("Contract instantiated at:", instantiateReceipt.contractAddress);
    // Upload successful, code ID: 11032
    // Contract instantiated at: neutron15e8pmchvjyx3uecc9zqt82c90vxghgxcu6chexwuwggp4yh5tdkq36mc5w
}

main().catch(console.error);
