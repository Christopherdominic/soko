import { Suspense } from "react";
import { AppShell } from "@/components/AppShell";
import { Loading } from "@/components/ui";
import { PaymentView } from "@/components/PaymentView";

export default function PaymentPage() {
  return (
    <AppShell>
      <Suspense fallback={<Loading />}>
        <PaymentView />
      </Suspense>
    </AppShell>
  );
}
