"use client"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

interface TokenInputProps {
  onTokenChange: (value: string) => void
  label?: string
}

export function TokenInput({ onTokenChange, label = "Token Address" }: TokenInputProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor="tokenAddress">{label}</Label>
      <Input id="tokenAddress" placeholder="Enter token address" onChange={(e) => onTokenChange(e.target.value)} />
    </div>
  )
}

