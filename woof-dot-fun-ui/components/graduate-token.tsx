"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Window as KeplrWindow } from "@keplr-wallet/types";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { GasPrice } from "@cosmjs/stargate"
import { TokenInput } from "./token-input"

declare global {
  interface Window extends KeplrWindow {}
}

export function GraduateToken() {
  const [token, setToken] = useState("")
  const CONTRACT_ADDRESS = "neutron1y8l8egyqlhnq4h9ph3ggrfwx6hr5vam9dn6t8a350z80hcqjjwus4ckaqe";
  const CHAIN_ID = "pion-1";
  const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";
  const savedAddress = localStorage.getItem("connectedAddress")!

  const handleGraduate = async () => {
    console.log("Graduate token:", token)
    const msg = {
      graduate: {
        token_address: token
      },
    }

    if (!window.keplr || !savedAddress) {
      throw new Error("Please connect your wallet first");
    }

    // Get offline signer from Keplr
    const offlineSigner = window.keplr.getOfflineSigner(CHAIN_ID);
    
    // Create a CosmWasm client
    const client = await SigningCosmWasmClient.connectWithSigner(
      RPC_ENDPOINT,
      offlineSigner,
      { gasPrice: GasPrice.fromString("0.025untrn") }
    );

    // Execute the transaction
    const result = await client.execute(
      savedAddress,
      CONTRACT_ADDRESS,
      msg,
      "auto",
    );
    
    console.log("Graduation successfully. ", result);
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle>Graduate Token</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <TokenInput onTokenChange={setToken} />
        <Button onClick={() => handleGraduate()} className="w-full h-12 text-lg font-medium bg-purple-600 hover:bg-purple-700">
          Graduate Token
        </Button>
      </CardContent>
    </Card>
  )
}
