import { Suspense } from "react";
import { RequireMerchant } from "@/hooks/useMerchant";
import { AppHeader } from "@/components/AppHeader";
import { MerchantReceiptView } from "@/components/MerchantReceiptView";

export default function MerchantReceiptPage() {
  return (
    <RequireMerchant>
      <AppHeader />
      <Suspense fallback={<p className="p-6 text-center text-muted">Loading…</p>}>
        <MerchantReceiptView />
      </Suspense>
    </RequireMerchant>
  );
}
