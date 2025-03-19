"use client"

import { useState } from "react"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Search, SlidersHorizontal } from "lucide-react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"

export function TokenSearch() {
  const [open, setOpen] = useState(false)
  const [searchParams, setSearchParams] = useState({
    name: "",
    symbol: "",
    minMarketCap: "",
    maxMarketCap: "",
    category: "",
  })

  const handleSearch = () => {
    console.log("Searching with params:", searchParams)
    // Implement the search logic here
    setOpen(false)
  }

  return (
    <div className="flex gap-4">
      <div className="relative flex-1">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-500" />
        <Input placeholder="Search for tokens..." className="pl-10 bg-gray-900 border-gray-800" />
      </div>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" className="border-gray-800 text-gray-400 hover:text-white hover:bg-gray-800">
            <SlidersHorizontal className="mr-2 h-4 w-4" />
            Advanced Search
          </Button>
        </DialogTrigger>
        <DialogContent className="sm:max-w-[425px] bg-gray-900 text-white">
          <DialogHeader>
            <DialogTitle>Advanced Search</DialogTitle>
            <DialogDescription>Refine your token search with additional parameters.</DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="name" className="text-right">
                Name
              </Label>
              <Input
                id="name"
                value={searchParams.name}
                onChange={(e) => setSearchParams({ ...searchParams, name: e.target.value })}
                className="col-span-3"
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="symbol" className="text-right">
                Symbol
              </Label>
              <Input
                id="symbol"
                value={searchParams.symbol}
                onChange={(e) => setSearchParams({ ...searchParams, symbol: e.target.value })}
                className="col-span-3"
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="minMarketCap" className="text-right">
                Min Market Cap
              </Label>
              <Input
                id="minMarketCap"
                value={searchParams.minMarketCap}
                onChange={(e) => setSearchParams({ ...searchParams, minMarketCap: e.target.value })}
                className="col-span-3"
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="maxMarketCap" className="text-right">
                Max Market Cap
              </Label>
              <Input
                id="maxMarketCap"
                value={searchParams.maxMarketCap}
                onChange={(e) => setSearchParams({ ...searchParams, maxMarketCap: e.target.value })}
                className="col-span-3"
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="category" className="text-right">
                Category
              </Label>
              <Select
                onValueChange={(value) => setSearchParams({ ...searchParams, category: value })}
                defaultValue={searchParams.category}
              >
                <SelectTrigger className="col-span-3">
                  <SelectValue placeholder="Select a category" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="defi">DeFi</SelectItem>
                  <SelectItem value="nft">NFT</SelectItem>
                  <SelectItem value="gaming">Gaming</SelectItem>
                  <SelectItem value="dao">DAO</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <Button onClick={handleSearch} className="w-full">
            Search
          </Button>
        </DialogContent>
      </Dialog>
    </div>
  )
}

