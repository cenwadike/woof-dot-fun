"use client"

import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { XCircle } from "lucide-react"
import { toast } from "@/components/ui/use-toast"
import { Window as KeplrWindow } from "@keplr-wallet/types";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { GasPrice } from "@cosmjs/stargate"
import { TokenInput } from "./token-input"

declare global {
  interface Window extends KeplrWindow {}
}

type Order = {
  id: string
  pairId: string
  type: "Buy" | "Sell"
  price: number
  amount: number
  time: string
}

export function CancelOrder() {
  const [orders, setOrders] = useState<Order[]>([])
  const [selectedOrder, setSelectedOrder] = useState<string | null>(null)
  const [tokenAddress, setTokenAddress] = useState<string | null>(null)
  const CONTRACT_ADDRESS = "neutron1y8l8egyqlhnq4h9ph3ggrfwx6hr5vam9dn6t8a350z80hcqjjwus4ckaqe";
  const CHAIN_ID = "pion-1";
  const RPC_ENDPOINT = "https://rpc-palvus.pion-1.ntrn.tech";

  const savedAddress = localStorage.getItem("connectedAddress")!

  useEffect(() => {
    const fetchOrders = async () => {
      if (!window.keplr || !savedAddress) {
        throw new Error("Please connect your wallet first");
      }

      const query = {
        get_user_orders: {
          address: savedAddress
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
        
        setOrders(res)
      } catch (error) {
        // This is mock data for demonstration
        const mockOrders: Order[] = [
          { id: "0001", pairId: "HUAHUA-ATOM", type: "Buy", price: 1.0, amount: 500, time: "2023-12-01 14:30:00" },
          { id: "0002", pairId: "HUAHUA-OSMO", type: "Sell", price: 1.02, amount: 300, time: "2023-12-01 14:29:30" },
          { id: "0003", pairId: "HUAHUA-JUNO", type: "Buy", price: 0.99, amount: 1000, time: "2023-12-01 14:29:00" },
        ]
        setOrders(mockOrders)
      }

      if (orders.length < 1) {
        const mockOrders: Order[] = [
          { id: "0001", pairId: "HUAHUA-ATOM", type: "Buy", price: 1.0, amount: 500, time: "2023-12-01 14:30:00" },
          { id: "0002", pairId: "HUAHUA-OSMO", type: "Sell", price: 1.02, amount: 300, time: "2023-12-01 14:29:30" },
          { id: "0003", pairId: "HUAHUA-JUNO", type: "Buy", price: 0.99, amount: 1000, time: "2023-12-01 14:29:00" },
        ]
        setOrders(mockOrders)
      }
    }

    fetchOrders()
  }, [])

  const handleCancelOrder = async() => {
    if (!selectedOrder) {
      toast({
        title: "Error",
        description: "Please select an order to cancel",
        variant: "destructive",
      })
      return
    }

    const orderToCancel = orders.find((order) => order.id === selectedOrder)
    if (orderToCancel) {
      console.log("Cancelling order:", orderToCancel)
      const msg = {
        cancel_order: {
          order_id: orderToCancel.id,
          pair_id: orderToCancel.pairId,
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
      
      console.log("Swap successfully. ", result);

      toast({
        title: "Order Cancelled",
        description: `Order ${selectedOrder} (${orderToCancel.pairId}) has been cancelled successfully`,
      })
      setSelectedOrder(null)
    }
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle className="text-xl flex items-center">
          <XCircle className="mr-2 h-5 w-5 text-red-400" />
          Cancel Order
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">

        <div className="space-y-6">
          <TokenInput onTokenChange={setTokenAddress} />
        </div>
        <Select value={selectedOrder || undefined} onValueChange={setSelectedOrder}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder="Select an order to cancel" />
          </SelectTrigger>
          <SelectContent>
            {orders.map((order) => (
              <SelectItem key={order.id} value={order.id}>
                <div className="flex justify-between items-center w-full">
                  <span>{order.pairId}</span>
                  <span className={order.type === "Buy" ? "text-green-400" : "text-red-400"}>
                    {order.type} {order.amount.toFixed(2)} @ {order.price.toFixed(4)}
                  </span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          onClick={handleCancelOrder}
          className="w-full h-12 text-lg font-medium bg-red-600 hover:bg-red-700 transition-colors"
          disabled={!selectedOrder}
        >
          Cancel Selected Order
        </Button>
      </CardContent>
    </Card>
  )
}

