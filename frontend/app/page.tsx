import { RequireMerchant } from "@/hooks/useMerchant";
import { AppHeader } from "@/components/AppHeader";
import { PrimaryLink } from "@/components/ui";
import { RecentPayments } from "@/components/RecentPayments";
import { SystemStatus } from "@/components/SystemStatus";

export default function Home() {
  return (
    <RequireMerchant>
      <AppHeader />
      <main className="mx-auto w-full max-w-2xl flex-1 space-y-8 px-4 py-6">
        <PrimaryLink href="/receive">＋ Receive payment</PrimaryLink>

        <section>
          <h2 className="mb-3 text-lg font-bold">Recent payments</h2>
          <RecentPayments />
        </section>

        <section>
          <h2 className="mb-3 text-lg font-bold">System status</h2>
          <SystemStatus />
        </section>
      </main>
    </RequireMerchant>
  );
}
