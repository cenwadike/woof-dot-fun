"use client"

import { useState, useEffect } from "react"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"

type TokenBalance = {
  symbol: string
  balance: string
}

export function TokenBalance() {
  const [searchTerm, setSearchTerm] = useState("")
  const [searchedTokens, setSearchedTokens] = useState<Set<string>>(new Set(["HUAHUA"]))
  const [balances, setBalances] = useState<TokenBalance[]>([])

  useEffect(() => {
    // Load searched tokens from localStorage
    const savedTokens = localStorage.getItem("searchedTokens")
    if (savedTokens) {
      setSearchedTokens(new Set(JSON.parse(savedTokens)))
    }
  }, [])

  useEffect(() => {
    const fetchBalances = async () => {
      // In a real application, you would fetch this data from your blockchain or API
      // This is mock data for demonstration
      const mockBalances: TokenBalance[] = Array.from(searchedTokens).map((token) => ({
        symbol: token,
        balance: (Math.random() * 1000).toFixed(2),
      }))
      setBalances(mockBalances)
    }

    fetchBalances()
  }, [searchedTokens])

  const handleSearch = () => {
    if (searchTerm && !searchedTokens.has(searchTerm.toUpperCase())) {
      const newSearchedTokens = new Set(searchedTokens)
      newSearchedTokens.add(searchTerm.toUpperCase())
      setSearchedTokens(newSearchedTokens)
      localStorage.setItem("searchedTokens", JSON.stringify(Array.from(newSearchedTokens)))
      setSearchTerm("")
    }
  }

  return (
    <Card className="bg-gray-900 border-gray-800">
      <CardHeader>
        <CardTitle className="text-2xl">Token Balances</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex space-x-2 mb-4">
          <Input
            placeholder="Search tokens..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onKeyPress={(e) => e.key === "Enter" && handleSearch()}
          />
          <Button onClick={handleSearch}>Search</Button>
        </div>
        <ScrollArea className="h-[300px] pr-4">
          <div className="space-y-4">
            {balances.map((token) => (
              <div key={token.symbol} className="flex justify-between items-center p-3 bg-gray-800 rounded-lg">
                <span className="font-medium text-green-400">{token.symbol}</span>
                <span className="text-lg">{token.balance}</span>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  )
}

