"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

export function SystemStats() {
  // Mock data - replace with actual query results
  const stats = {
    totalPairs: 156,
    totalOrders: 15234,
    totalTrades: 45678,
    totalVolume: "1,234,567",
    totalUsers: 5678,
    totalFeesCollected: "12,345",
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle>System Statistics</CardTitle>
        <CardDescription>Overall platform metrics</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
          <div>
            <p className="text-sm text-gray-400">Total Pairs</p>
            <p className="text-2xl font-bold">{stats.totalPairs}</p>
          </div>
          <div>
            <p className="text-sm text-gray-400">Total Orders</p>
            <p className="text-2xl font-bold">{stats.totalOrders}</p>
          </div>
          <div>
            <p className="text-sm text-gray-400">Total Trades</p>
            <p className="text-2xl font-bold">{stats.totalTrades}</p>
          </div>
          <div>
            <p className="text-sm text-gray-400">Volume (HUAHUA)</p>
            <p className="text-2xl font-bold">{stats.totalVolume}</p>
          </div>
          <div>
            <p className="text-sm text-gray-400">Total Users</p>
            <p className="text-2xl font-bold">{stats.totalUsers}</p>
          </div>
          <div>
            <p className="text-sm text-gray-400">Fees Collected</p>
            <p className="text-2xl font-bold">{stats.totalFeesCollected}</p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

