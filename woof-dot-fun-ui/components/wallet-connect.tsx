"use client"

import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Wallet, ChevronDown } from "lucide-react"

import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice } from "@cosmjs/stargate";

import { Window as KeplrWindow } from "@keplr-wallet/types";

declare global {
  interface Window extends KeplrWindow {}
}

export function WalletConnect() {
  const [connected, setConnected] = useState(false)
  const [address, setAddress] = useState<string | null>(null)

  // Neutron testnet configuration
  const CHAIN_ID = "pion-1";
  const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";

  // Chain configuration for Keplr
  const chainConfig = {
    chainId: CHAIN_ID,
    chainName: 'Neutron Testnet',
    rpc: RPC_ENDPOINT,
    rest: 'https://rest-palvus.pion-1.ntrn.tech',
    bip44: {
      coinType: 118,
    },
    bech32Config: {
      bech32PrefixAccAddr: 'neutron',
      bech32PrefixAccPub: 'neutronpub',
      bech32PrefixValAddr: 'neutronvaloper',
      bech32PrefixValPub: 'neutronvaloperpub',
      bech32PrefixConsAddr: 'neutronvalcons',
      bech32PrefixConsPub: 'neutronvalconspub',
    },
    currencies: [
      {
        coinDenom: 'NTRN',
        coinMinimalDenom: 'untrn',
        coinDecimals: 6,
      },
    ],
    feeCurrencies: [
      {
        coinDenom: 'NTRN',
        coinMinimalDenom: 'untrn',
        coinDecimals: 6,
      },
    ],
    stakeCurrency: {
      coinDenom: 'NTRN',
      coinMinimalDenom: 'untrn',
      coinDecimals: 6,
    },
    gasPrices: '0.025untrn',
    gasAdjustment: 1.3,
  };

  useEffect(() => {
    const savedAddress = localStorage.getItem("connectedAddress")
    if (savedAddress) {
      setConnected(true)
      setAddress(savedAddress)
    }
  }, [])

  const connectWallet = async (walletId: string) => {
    try {
      if (!window.keplr) {
        throw new Error("Please install Keplr extension");
      }

      // Suggest chain to Keplr
      await window.keplr.experimentalSuggestChain(chainConfig);

      // Enable access to Keplr
      await window.keplr.enable(CHAIN_ID);

      const key = await window.keplr.getKey(CHAIN_ID);
      setAddress(key.bech32Address);
      setConnected(true)
      localStorage.setItem("connectedAddress", key.bech32Address)
    } catch (error) {
      console.error(error);
      alert(`Failed to connect to ${walletId}. Please make sure the extension is installed and try again.`)
    }
  }

  const disconnectWallet = () => {
    setConnected(false)
    setAddress(null)
    localStorage.removeItem("connectedAddress")
  }

  if (connected && address) {
    return (
      <div className="relative group">
        <button
          onClick={disconnectWallet}
          className="flex items-center px-3 py-2 bg-green-600/20 hover:bg-green-600/30 transition-colors rounded-full text-green-400 border border-green-600/30"
        >
          <div className="flex items-center gap-2">
            <Wallet className="h-4 w-4" />
            <span className="font-medium">
              {address.slice(0, 6)}...{address.slice(-4)}
            </span>
          </div>
        </button>
        <div className="absolute inset-0 -z-10 bg-green-600/10 blur-md rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
      </div>
    )
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" className="rounded-full">
          <Wallet className="mr-2 h-4 w-4" /> Connect Wallet
          <ChevronDown className="ml-2 h-4 w-4 opacity-50" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onSelect={() => connectWallet("keplr")}>Keplr</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => connectWallet("leap")}>Leap</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => connectWallet("cosmostation")}>Cosmostation</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

