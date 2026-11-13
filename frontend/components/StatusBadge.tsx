import { CheckCircle2, Clock3, MinusCircle, XCircle } from "lucide-react";
import type { PaymentStatus } from "@/types/api";

const STYLES: Record<PaymentStatus, { label: string; Icon: typeof Clock3; cls: string }> = {
  pending: { label: "Waiting", Icon: Clock3, cls: "bg-amber-50 text-amber-800 ring-amber-200" },
  paid: { label: "Paid", Icon: CheckCircle2, cls: "bg-emerald-50 text-emerald-800 ring-emerald-200" },
  expired: { label: "Expired", Icon: MinusCircle, cls: "bg-stone-100 text-stone-600 ring-stone-200" },
  failed: { label: "Failed", Icon: XCircle, cls: "bg-red-50 text-red-800 ring-red-200" },
};

export function StatusBadge({ status }: { status: PaymentStatus }) {
  const { label, Icon, cls } = STYLES[status];
  return (
    <span className={`inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs font-semibold ring-1 ring-inset ${cls}`}>
      <Icon aria-hidden className="h-3.5 w-3.5" />
      {label}
    </span>
  );
}
