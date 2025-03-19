"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { TokenInput } from "@/components/token-input"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ArrowRightLeft } from "lucide-react"
import { Window as KeplrWindow } from "@keplr-wallet/types";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { GasPrice } from "@cosmjs/stargate"

declare global {
  interface Window extends KeplrWindow {}
}

export function Swap() {
  const [tokenAddress, setTokenAddress] = useState("")
  const [amount, setAmount] = useState("")
  const [minReturn, setMinReturn] = useState("")
  const [activeTab, setActiveTab] = useState("buy")
  const CONTRACT_ADDRESS = "neutron1y8l8egyqlhnq4h9ph3ggrfwx6hr5vam9dn6t8a350z80hcqjjwus4ckaqe";
  const CHAIN_ID = "pion-1";
  const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";

  const savedAddress = localStorage.getItem("connectedAddress")
 
  const getTokenPoolInfo = async(tokenAddress: String) => {
  
    if (!window.keplr || !savedAddress) {
      throw new Error("Please connect your wallet first");
    }

    const query = {
      get_pool: {
        token_address: tokenAddress
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

    const res = await client.queryContractSmart(
      CONTRACT_ADDRESS,
      query
    );

    return res.pool
  }

  const handleSwap = async (isBuy: boolean) => {
    console.log("Swap:", { tokenAddress, amount, minReturn, isBuy })
  
    if (!window.keplr || !savedAddress) {
      throw new Error("Please connect your wallet first");
    }

    const formatted_amount = parseInt(amount) * 1000000;
    if (formatted_amount <= 0) {
      throw new Error("Amount must be greater than 0");
    }
    const string_formatted_amount = formatted_amount.toString();

    const pool = await getTokenPoolInfo(tokenAddress);

    const msg = {
      swap: {
        pair_id: pool.pair_id,
        token_address: tokenAddress,
        amount: string_formatted_amount,
        min_return: minReturn,
        order_type: isBuy ? "Buy" : "Sell",
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
    
    console.log("Swap successfully. ", result);
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle className="flex items-center text-xl">
          <ArrowRightLeft className="mr-2 h-5 w-5 text-primary" />
          Swap
        </CardTitle>
      </CardHeader>
      <CardContent>
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-2 mb-6">
            <TabsTrigger
              value="buy"
              className={`${activeTab === "buy" ? "bg-green-600 text-white" : "bg-gray-800"} transition-colors`}
            >
              Buy
            </TabsTrigger>
            <TabsTrigger
              value="sell"
              className={`${activeTab === "sell" ? "bg-red-600 text-white" : "bg-gray-800"} transition-colors`}
            >
              Sell
            </TabsTrigger>
          </TabsList>
          <div className="space-y-6">
            <TokenInput onTokenChange={setTokenAddress} />
            <div className="space-y-2">
              <Label htmlFor="amount">Amount</Label>
              <div className="relative">
                <Input
                  id="amount"
                  type="number"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="0.00"
                  className="pr-20"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-gray-400">
                  {activeTab === "buy" ? "Huahua" : "Token"}
                </span>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="minReturn">Minimum Return</Label>
              <div className="relative">
                <Input
                  id="minReturn"
                  type="number"
                  value={minReturn}
                  onChange={(e) => setMinReturn(e.target.value)}
                  placeholder="0.00"
                  className="pr-20"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-gray-400">
                  {activeTab === "buy" ? "Token" : "Huahua"}
                </span>
              </div>
            </div>
          </div>
          <Button
            onClick={() => handleSwap(activeTab === "buy")}
            className={`w-full mt-6 h-12 text-lg font-medium ${
              activeTab === "buy" ? "bg-green-600 hover:bg-green-700" : "bg-red-600 hover:bg-red-700"
            }`}
          >
            {activeTab === "buy" ? "Buy" : "Sell"} Tokens
          </Button>
        </Tabs>
      </CardContent>
    </Card>
  )
}

