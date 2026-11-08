import type { PaymentStatus } from "@/types/api";

const STYLES: Record<PaymentStatus, { label: string; icon: string; cls: string }> = {
  pending: { label: "Waiting", icon: "◷", cls: "bg-amber-100 text-amber-900" },
  paid: { label: "Paid", icon: "✓", cls: "bg-green-100 text-green-900" },
  expired: { label: "Expired", icon: "–", cls: "bg-stone-200 text-stone-700" },
  failed: { label: "Failed", icon: "✕", cls: "bg-red-100 text-red-900" },
};

export function StatusBadge({ status }: { status: PaymentStatus }) {
  const s = STYLES[status];
  return (
    <span className={`inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs font-semibold ${s.cls}`}>
      <span aria-hidden>{s.icon}</span>
      {s.label}
    </span>
  );
}
