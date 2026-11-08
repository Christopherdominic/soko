import { Suspense } from "react";
import { RequireMerchant } from "@/hooks/useMerchant";
import { AppHeader } from "@/components/AppHeader";
import { HistoryView } from "@/components/HistoryView";

export default function HistoryPage() {
  return (
    <RequireMerchant>
      <AppHeader />
      <Suspense fallback={<p className="p-6 text-center text-muted">Loading…</p>}>
        <HistoryView />
      </Suspense>
    </RequireMerchant>
  );
}
