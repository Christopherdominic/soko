import { Suspense } from "react";
import { AppShell } from "@/components/AppShell";
import { Loading } from "@/components/ui";
import { HistoryView } from "@/components/HistoryView";

export default function HistoryPage() {
  return (
    <AppShell>
      <Suspense fallback={<Loading />}>
        <HistoryView />
      </Suspense>
    </AppShell>
  );
}
