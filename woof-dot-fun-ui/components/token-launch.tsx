"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Window as KeplrWindow } from "@keplr-wallet/types";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { GasPrice } from "@cosmjs/stargate"

declare global {
  interface Window extends KeplrWindow {}
}

export function TokenLaunch() {
  const [name, setName] = useState("")
  const [symbol, setSymbol] = useState("")
  const [decimals, setDecimals] = useState("6")
  const [uri, setUri] = useState("")
  const [maxPriceImpact, setMaxPriceImpact] = useState("10")
  const [curveSlope, setCurveSlope] = useState("500")

  const handleLaunch = async() => {
    console.log("Creating token:", { name, symbol, decimals, uri, maxPriceImpact, curveSlope })

    const CONTRACT_ADDRESS = "neutron127nn9qc24vq5qx0c0luheah9986rwe5wfm8tltmg9c2ym7c9gths4enrge";
    const CHAIN_ID = "pion-1";
    const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";

    const savedAddress = localStorage.getItem("connectedAddress")

    if (!window.keplr || !savedAddress) {
      throw new Error("Please connect your wallet first");
    }
    
    const msg = {
      create_token: {
        name,
        symbol,
        decimals: parseInt(decimals),
        uri,
        max_price_impact: maxPriceImpact,
        curve_slope: curveSlope,
      }
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

    console.log("Launch successful:", result);
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle className="text-2xl">Launch New Token</CardTitle>
        <CardDescription>Create your own token with custom parameters</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="name">Token Name</Label>
          <Input id="name" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. My Awesome Token" />
        </div>
        <div className="space-y-2">
          <Label htmlFor="symbol">Token Symbol</Label>
          <Input id="symbol" value={symbol} onChange={(e) => setSymbol(e.target.value)} placeholder="e.g. MAT" />
        </div>
        <div className="space-y-2">
          <Label htmlFor="decimals">Decimals</Label>
          <Input id="decimals" type="number" value={decimals} onChange={(e) => setDecimals(e.target.value)} />
        </div>
        <div className="space-y-2">
          <Label htmlFor="uri">Token Image URL</Label>
          <Input id="uri" value={uri} onChange={(e) => setUri(e.target.value)} placeholder="e.g. https://image/url" />
        </div>
        <div className="space-y-2">
          <Label htmlFor="maxPriceImpact">Max Price Impact</Label>
          <Input
            id="maxPriceImpact"
            type="number"
            value={maxPriceImpact}
            onChange={(e) => setMaxPriceImpact(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="curveSlope">Curve Slope</Label>
          <Input id="curveSlope" type="number" value={curveSlope} onChange={(e) => setCurveSlope(e.target.value)} />
        </div>
      </CardContent>
      <CardFooter>
        <Button onClick={handleLaunch} className="w-full h-12 text-lg font-medium bg-green-600 hover:bg-green-700">
          Launch Token
        </Button>
      </CardFooter>
    </Card>
  )
}

