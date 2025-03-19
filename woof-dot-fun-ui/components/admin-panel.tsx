"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Switch } from "@/components/ui/switch"
import { Window as KeplrWindow } from "@keplr-wallet/types";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { GasPrice } from "@cosmjs/stargate"

declare global {
  interface Window extends KeplrWindow {}
}

export function AdminPanel() {
  const [config, setConfig] = useState({
    tokenFactory: "",
    feeCollector: "",
    makerFeeRate: "",
    takerFeeRate: "",
    tradingFeeRate: "",
    quoteTokenTotalSupply: "",
    bondingCurveSupply: "",
    lpSupply: "",
    enabled: false,
  })
  const CONTRACT_ADDRESS = "neutron1y8l8egyqlhnq4h9ph3ggrfwx6hr5vam9dn6t8a350z80hcqjjwus4ckaqe";
  const CHAIN_ID = "pion-1";
  const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";
  const savedAddress = localStorage.getItem("connectedAddress")!

  const handleUpdateConfig = async() => {
    console.log("Update config:", config)
    const msg = {
      update_config: {
        token_factory: config.tokenFactory,
        fee_collector: config.feeCollector,
        maker_fee: config.makerFeeRate,
        taker_fee: config.takerFeeRate,
        quote_token_total_supply: config.quoteTokenTotalSupply,
        bonding_curve_supply: config.bondingCurveSupply,
        lp_supply: config.lpSupply,
        enabled: config.enabled,
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
    
    console.log("Config update successfully. ", result);
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle>Admin Panel</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {Object.entries(config).map(([key, value]) =>
          key === "enabled" ? (
            <div key={key} className="flex items-center justify-between">
              <Label htmlFor={key}>{key}</Label>
              <Switch
                id={key}
                checked={value as boolean}
                onCheckedChange={(checked) => setConfig({ ...config, [key]: checked })}
              />
            </div>
          ) : (
            <div key={key} className="space-y-2">
              <Label htmlFor={key}>{key}</Label>
              <Input
                id={key}
                value={value as string}
                onChange={(e) => setConfig({ ...config, [key]: e.target.value })}
                placeholder={`Enter ${key}`}
              />
            </div>
          ),
        )}
        <Button onClick={handleUpdateConfig} className="w-full h-12 text-lg font-medium bg-blue-600 hover:bg-blue-700">
          Update Config
        </Button>
      </CardContent>
    </Card>
  )
}

