"use client"

import { useState, useEffect } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { CheckCircle2, Clock } from "lucide-react"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"

type Trade = {
  id: string
  pairId: string
  orderId: string
  type: "Buy" | "Sell"
  price: number
  amount: number
  time: string
  executed: boolean
}

export function TradeHistory() {
  const [trades, setTrades] = useState<Trade[]>([])

  useEffect(() => {
    const fetchTrades = async () => {
      const mockTrades: Trade[] = [
        {
          id: "1",
          pairId: "HUAHUA-ATOM",
          orderId: "0001",
          type: "Buy",
          price: 1.0,
          amount: 500,
          time: "2023-12-01 14:30:00",
          executed: true,
        },
        {
          id: "2",
          pairId: "HUAHUA-OSMO",
          orderId: "0002",
          type: "Sell",
          price: 1.02,
          amount: 300,
          time: "2023-12-01 14:29:30",
          executed: false,
        },
        {
          id: "3",
          pairId: "HUAHUA-JUNO",
          orderId: "0003",
          type: "Buy",
          price: 0.99,
          amount: 1000,
          time: "2023-12-01 14:29:00",
          executed: true,
        },
        {
          id: "4",
          pairId: "ATOM-OSMO",
          orderId: "0004",
          type: "Sell",
          price: 25.5,
          amount: 50,
          time: "2023-12-01 14:28:30",
          executed: false,
        },
        {
          id: "5",
          pairId: "JUNO-ATOM",
          orderId: "0005",
          type: "Buy",
          price: 6.2,
          amount: 200,
          time: "2023-12-01 14:28:00",
          executed: true,
        },
      ]
      setTrades(mockTrades)
    }

    fetchTrades()
  }, [])

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle className="text-2xl font-bold">Trade History</CardTitle>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Status</TableHead>
              <TableHead>Pair ID</TableHead>
              <TableHead>Order ID</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Price</TableHead>
              <TableHead>Amount</TableHead>
              <TableHead>Time</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {trades.map((trade) => (
              <TableRow key={trade.id}>
                <TableCell>
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger>
                        {trade.executed ? (
                          <CheckCircle2 className="h-5 w-5 text-green-500" />
                        ) : (
                          <Clock className="h-5 w-5 text-yellow-500 animate-pulse" />
                        )}
                      </TooltipTrigger>
                      <TooltipContent>
                        <p>{trade.executed ? "Executed" : "Pending"}</p>
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                </TableCell>
                <TableCell>{trade.pairId}</TableCell>
                <TableCell>{trade.orderId}</TableCell>
                <TableCell className={trade.type === "Buy" ? "text-green-400" : "text-red-400"}>{trade.type}</TableCell>
                <TableCell>{trade.price.toFixed(4)}</TableCell>
                <TableCell>{trade.amount.toFixed(2)}</TableCell>
                <TableCell>{trade.time}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  )
}

