import { Swap } from "@/components/swap"
import { LimitOrder } from "@/components/limit-order"
import { OrderBook } from "@/components/order-book"
import { TradeHistory } from "@/components/trade-history"
import { TokenBalance } from "@/components/token-balance"
import { TokenLaunch } from "@/components/token-launch"
import { TokenSearch } from "@/components/token-search"
import { SystemStatsShowcase } from "@/components/system-stats-showcase"
import { CancelOrder } from "@/components/cancel-order"
import { GraduateToken } from "@/components/graduate-token"
import { AdminPanel } from "@/components/admin-panel"
import { WalletConnect } from "@/components/wallet-connect"

export default function Home() {
  return (
    <main className="min-h-screen bg-gray-950 text-white">
      <header className="bg-gray-900 border-b border-gray-800 sticky top-0 z-50">
        <div className="container mx-auto px-4 py-4 flex justify-between items-center">
          <h1 className="text-3xl font-bold text-green-400">woof.fun</h1>
          <WalletConnect />
        </div>
      </header>

      <div className="container mx-auto px-4 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div className="lg:col-span-2 space-y-8">
            <TokenSearch />
            <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
              <Swap />
              <LimitOrder />
            </div>
            <OrderBook />
            <TradeHistory />
          </div>
          <div className="space-y-8">
            <TokenBalance />
            <CancelOrder />
            <TokenLaunch />
            <GraduateToken />
          </div>
        </div>

        <section className="mt-12">
          <h2 className="text-2xl font-semibold mb-4 text-green-400">DEX Analytics</h2>
          <SystemStatsShowcase />
        </section>

        <section className="mt-12">
          <h2 className="text-2xl font-semibold mb-4 text-green-400">Admin Panel</h2>
          <AdminPanel />
        </section>
      </div>
    </main>
  )
}

