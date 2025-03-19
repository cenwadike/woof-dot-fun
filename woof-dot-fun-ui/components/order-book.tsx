"use client"

import { useEffect, useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Window as KeplrWindow } from "@keplr-wallet/types";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { GasPrice } from "@cosmjs/stargate"

declare global {
  interface Window extends KeplrWindow {}
}

type Order = {
  price: number
  amount: number
  total: number
}

export function OrderBook() {
  const [buyOrders, setBuyOrders] = useState<Order[]>([])
  const [sellOrders, setSellOrders] = useState<Order[]>([])
  const [tokenAddress, setTokenAddress] = useState<string | null>(null)
  const CONTRACT_ADDRESS = "neutron1y8l8egyqlhnq4h9ph3ggrfwx6hr5vam9dn6t8a350z80hcqjjwus4ckaqe";
  const CHAIN_ID = "pion-1";
  const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";
  const savedAddress = localStorage.getItem("connectedAddress")!

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

  useEffect(() => {
    // Simulating fetching data from blockchain
    const fetchOrderBook = async () => {

      // Replace this with actual blockchain data fetching
      if (!window.keplr || !savedAddress) {
        throw new Error("Please connect your wallet first");
      }
    
      const pool = await getTokenPoolInfo(tokenAddress!);
      const query = {
        get_order_book: {
          pair_id: pool.pair_id
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
    
      try {
        const res = await client.queryContractSmart(
          CONTRACT_ADDRESS,
          query
        );
        const buys = [];
        const sells = [];

        setBuyOrders(res.map((r: any) => {
          buys.push(
            {price: r.last_price, amount: r.bids.length, total: r.base_volume_24h}
          )
        }))

        setSellOrders(res.map((r: any) => {
          sells.push(
            {price: r.last_price, amount: r.asks.length, total: r.quote_volume_24h}
          )
        }))
      } catch (error) {
        const mockBuyOrders: Order[] = [
          { price: 0.95, amount: 1000, total: 950 },
          { price: 0.94, amount: 1500, total: 1410 },
          { price: 0.93, amount: 2000, total: 1860 },
        ]
        const mockSellOrders: Order[] = [
          { price: 1.05, amount: 800, total: 840 },
          { price: 1.06, amount: 1200, total: 1272 },
          { price: 1.07, amount: 1800, total: 1926 },
        ]
        setBuyOrders(mockBuyOrders)
        setSellOrders(mockSellOrders)        
      }
    }

    fetchOrderBook()
    // Set up an interval to fetch data periodically
    const interval = setInterval(fetchOrderBook, 10000) // Fetch every 10 seconds

    return () => clearInterval(interval)
  }, [])

  const maxTotal = Math.max(...buyOrders.map((order) => order.total), ...sellOrders.map((order) => order.total))

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle className="text-xl">Order Book</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <h3 className="text-lg font-semibold mb-2 text-green-500">Buy Orders</h3>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="text-right">Price</TableHead>
                  <TableHead className="text-right">Amount</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {buyOrders.map((order, index) => (
                  <TableRow key={index} style={{ position: "relative" }}>
                    <TableCell className="text-right text-green-400" style={{ position: "relative", zIndex: 1 }}>
                      {order.price.toFixed(4)}
                    </TableCell>
                    <TableCell className="text-right" style={{ position: "relative", zIndex: 1 }}>
                      {order.amount.toFixed(2)}
                    </TableCell>
                    <TableCell className="text-right" style={{ position: "relative", zIndex: 1 }}>
                      {order.total.toFixed(2)}
                    </TableCell>
                    <TableCell
                      className="absolute inset-0 bg-green-500 opacity-10"
                      style={{ width: `${(order.total / maxTotal) * 100}%`, zIndex: 0 }}
                    />
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <div>
            <h3 className="text-lg font-semibold mb-2 text-red-500">Sell Orders</h3>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="text-right">Price</TableHead>
                  <TableHead className="text-right">Amount</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {sellOrders.map((order, index) => (
                  <TableRow key={index} style={{ position: "relative" }}>
                    <TableCell className="text-right text-red-400" style={{ position: "relative", zIndex: 1 }}>
                      {order.price.toFixed(4)}
                    </TableCell>
                    <TableCell className="text-right" style={{ position: "relative", zIndex: 1 }}>
                      {order.amount.toFixed(2)}
                    </TableCell>
                    <TableCell className="text-right" style={{ position: "relative", zIndex: 1 }}>
                      {order.total.toFixed(2)}
                    </TableCell>
                    <TableCell
                      className="absolute inset-0 bg-red-500 opacity-10"
                      style={{ width: `${(order.total / maxTotal) * 100}%`, zIndex: 0 }}
                    />
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

