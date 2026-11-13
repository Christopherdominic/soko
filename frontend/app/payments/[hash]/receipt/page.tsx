import { Suspense } from "react";
import { AppShell } from "@/components/AppShell";
import { Loading } from "@/components/ui";
import { MerchantReceiptView } from "@/components/MerchantReceiptView";

export default function MerchantReceiptPage() {
  return (
    <AppShell>
      <Suspense fallback={<Loading />}>
        <MerchantReceiptView />
      </Suspense>
    </AppShell>
  );
}
