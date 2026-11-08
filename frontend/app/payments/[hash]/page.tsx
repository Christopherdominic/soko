import { Suspense } from "react";
import { RequireMerchant } from "@/hooks/useMerchant";
import { AppHeader } from "@/components/AppHeader";
import { PaymentView } from "@/components/PaymentView";

export default function PaymentPage() {
  return (
    <RequireMerchant>
      <AppHeader />
      <Suspense fallback={<p className="p-6 text-center text-muted">Loading…</p>}>
        <PaymentView />
      </Suspense>
    </RequireMerchant>
  );
}
