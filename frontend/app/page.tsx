import Link from "next/link";
import { RequireMerchant } from "@/hooks/useMerchant";
import { AppHeader } from "@/components/AppHeader";
import { PrimaryLink } from "@/components/ui";
import { DashboardStats } from "@/components/DashboardStats";
import { RecentPayments } from "@/components/RecentPayments";
import { SystemStatus } from "@/components/SystemStatus";

export default function Home() {
  return (
    <RequireMerchant>
      <AppHeader />
      <main className="mx-auto w-full max-w-2xl flex-1 space-y-8 px-4 py-6">
        <PrimaryLink href="/receive">＋ Receive payment</PrimaryLink>

        <section aria-labelledby="sales">
          <h2 id="sales" className="sr-only">Sales summary</h2>
          <DashboardStats />
        </section>

        <section>
          <div className="mb-3 flex items-baseline justify-between">
            <h2 className="text-lg font-bold">Recent transactions</h2>
            <Link href="/history" className="font-semibold text-forest underline-offset-4 hover:underline">
              See all
            </Link>
          </div>
          <RecentPayments limit={5} />
        </section>

        <details className="group">
          <summary className="cursor-pointer text-sm font-semibold text-muted">System status</summary>
          <div className="mt-3">
            <SystemStatus />
          </div>
        </details>
      </main>
    </RequireMerchant>
  );
}
