import { Suspense } from "react";
import { PublicReceiptView } from "@/components/PublicReceiptView";

export default function PublicReceiptPage() {
  return (
    <Suspense fallback={<p className="p-6 text-center text-muted">Loading…</p>}>
      <PublicReceiptView />
    </Suspense>
  );
}
