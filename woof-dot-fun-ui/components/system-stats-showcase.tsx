"use client"

import { useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { ChevronDown } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"

const stats = [
  { label: "Total Pairs", value: "156" },
  { label: "Total Orders", value: "15,234" },
  { label: "Total Trades", value: "45,678" },
  { label: "Total Volume", value: "1,234,567 WOOF" },
  { label: "Total Users", value: "5,678" },
  { label: "Total Fees Collected", value: "12,345 WOOF" },
]

export function SystemStatsShowcase() {
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null)

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {stats.map((stat, index) => (
        <Card key={stat.label} className="bg-gray-900 border-gray-800">
          <CardContent className="p-6">
            <motion.div
              className="cursor-pointer"
              whileHover={{ scale: 1.05 }}
              onClick={() => setExpandedIndex(expandedIndex === index ? null : index)}
            >
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-semibold text-green-400">{stat.label}</h3>
                <Button variant="ghost" size="sm">
                  <ChevronDown
                    className={`h-4 w-4 transition-transform duration-200 ${
                      expandedIndex === index ? "transform rotate-180" : ""
                    }`}
                  />
                </Button>
              </div>
              <p className="text-3xl font-bold mt-2">{stat.value}</p>
              <AnimatePresence>
                {expandedIndex === index && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                    exit={{ opacity: 0, height: 0 }}
                    transition={{ duration: 0.3 }}
                    className="mt-4 text-sm text-gray-400"
                  >
                    This is a placeholder for more detailed information about {stat.label.toLowerCase()}. In a real
                    application, you could include charts, trends, or additional metrics here.
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

