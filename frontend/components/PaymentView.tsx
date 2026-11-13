"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState, type ReactNode } from "react";
import QRCode from "qrcode";
import { Check, CheckCircle2, ChevronLeft, Copy, MinusCircle, QrCode, ReceiptText, XCircle } from "lucide-react";
import { api, ApiError } from "@/lib/api";
import { formatDateTime, formatNumber } from "@/lib/format";
import { StatusBadge } from "@/components/StatusBadge";
import { Button, ButtonLink, Card, Loading } from "@/components/ui";
import type { Payment } from "@/types/api";

const POLL_MS = 2000;

export function PaymentView() {
  const { hash } = useParams<{ hash: string }>();
  const [payment, setPayment] = useState<Payment | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Poll the backend while pending. The backend verifies with LND; we only display.
  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const load = async () => {
      try {
        const p = await api.payment(hash);
        if (cancelled) return;
        setPayment(p);
        setError(null);
        if (p.status === "pending") timer = setTimeout(load, POLL_MS);
      } catch (e) {
        if (cancelled) return;
        if (e instanceof ApiError && (e.status === 404 || e.status === 400)) {
          setError("Payment not found.");
        } else {
          setError("Connection problem. Retrying…");
          timer = setTimeout(load, POLL_MS * 2);
        }
      }
    };
    load();
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [hash]);

  if (!payment) {
    return error ? <p className="py-16 text-center text-muted">{error}</p> : <Loading />;
  }

  return (
    <>
      <Link href="/history" className="mb-4 inline-flex items-center gap-1 text-sm font-medium text-muted hover:text-charcoal">
        <ChevronLeft aria-hidden className="h-4 w-4" /> Transactions
      </Link>

      {payment.status === "pending" && <PendingView payment={payment} />}
      {payment.status === "paid" && <PaidView payment={payment} />}
      {payment.status === "expired" && (
        <ClosedView
          payment={payment}
          icon={<MinusCircle aria-hidden className="h-7 w-7" />}
          title="This payment request expired"
          text="No money was received. Create a new request to try again."
        />
      )}
      {payment.status === "failed" && (
        <ClosedView
          payment={payment}
          icon={<XCircle aria-hidden className="h-7 w-7" />}
          title="This payment could not be completed"
          text="No receipt was issued. Create a new request to try again."
        />
      )}
      {error && <p className="mt-4 text-center text-sm text-amber-800">{error}</p>}
    </>
  );
}

function PendingView({ payment }: { payment: Payment }) {
  const [qr, setQr] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const secondsLeft = useSecondsUntil(payment.expires_at);
  const total = Math.max(1, (new Date(payment.expires_at).getTime() - new Date(payment.created_at).getTime()) / 1000);
  const pct = Math.max(0, Math.min(100, (secondsLeft / total) * 100));

  useEffect(() => {
    // Uppercase BOLT11 fits QR alphanumeric mode, giving a smaller, easier-to-scan code.
    QRCode.toDataURL(`lightning:${payment.payment_request}`.toUpperCase(), {
      margin: 0,
      width: 720,
      errorCorrectionLevel: "M",
      color: { dark: "#142D27", light: "#FFFFFF" },
    }).then(setQr);
  }, [payment.payment_request]);

  const copy = async () => {
    await navigator.clipboard.writeText(payment.payment_request);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,420px)_minmax(0,1fr)] lg:items-start">
      <Card className="p-5 sm:p-7">
        <div className="relative mx-auto aspect-square w-full max-w-sm">
          {qr ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img src={qr} alt="Lightning payment QR code" className="h-full w-full" />
          ) : (
            <div className="h-full w-full animate-pulse rounded-xl bg-ivory" />
          )}
        </div>
        <p className="mt-5 text-center text-sm text-muted">Scan with any Lightning wallet</p>
      </Card>

      <Card className="p-5 sm:p-7">
        <div className="flex items-center gap-2 text-sm font-semibold text-amber-800" aria-live="polite">
          <span className="relative flex h-2.5 w-2.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-amber-400 opacity-60" />
            <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-amber-500" />
          </span>
          Waiting for payment
        </div>
        <p className="tabular mt-4 font-heading text-4xl font-extrabold text-forest sm:text-5xl">
          {formatNumber(payment.amount_sats)} <span className="text-2xl font-bold text-muted">sats</span>
        </p>
        {payment.description && <p className="mt-1 text-lg text-muted">{payment.description}</p>}

        <div className="mt-6">
          <div className="flex justify-between text-sm">
            <span className="text-muted">Expires in</span>
            <span className="tabular font-semibold">{formatCountdown(secondsLeft)}</span>
          </div>
          <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-ivory" aria-hidden>
            <div className="h-full rounded-full bg-forest transition-[width] duration-1000 ease-linear" style={{ width: `${pct}%` }} />
          </div>
        </div>

        <Button variant="secondary" size="lg" full onClick={copy} className="mt-6">
          {copied ? <Check aria-hidden className="h-5 w-5 text-emerald-700" /> : <Copy aria-hidden className="h-5 w-5" />}
          {copied ? "Copied" : "Copy payment request"}
        </Button>
        <p className="mt-4 text-sm text-muted">
          This screen updates on its own. Soko marks the payment as paid only after your Lightning node confirms it.
        </p>
      </Card>
    </div>
  );
}

function PaidView({ payment }: { payment: Payment }) {
  return (
    <Card className="mx-auto max-w-xl overflow-hidden">
      <div className="bg-emerald-50 px-6 py-10 text-center" role="status">
        <CheckCircle2 aria-hidden className="mx-auto h-16 w-16 text-emerald-600" strokeWidth={1.75} />
        <p className="mt-4 font-heading text-2xl font-extrabold text-emerald-900">Payment received</p>
        <p className="tabular mt-1 text-lg text-emerald-900/80">{formatNumber(payment.amount_sats)} sats</p>
        <p className="mt-2 text-sm text-emerald-900/70">Confirmed by your Lightning node</p>
      </div>
      <dl className="divide-y divide-line">
        <Row label="Receipt number" value={<span className="font-mono">{payment.receipt_number ?? "Issuing…"}</span>} />
        {payment.description && <Row label="For" value={payment.description} />}
        {payment.settled_at && <Row label="Paid at" value={formatDateTime(payment.settled_at)} />}
        <Row label="Status" value={<StatusBadge status="paid" />} />
      </dl>
      <div className="grid gap-3 border-t border-line p-5 sm:grid-cols-2">
        <ButtonLink href={`/payments/${payment.payment_hash}/receipt`} variant="secondary" size="lg" full>
          <ReceiptText aria-hidden className="h-5 w-5" /> View &amp; share receipt
        </ButtonLink>
        <ButtonLink href="/receive" size="lg" full>
          <QrCode aria-hidden className="h-5 w-5" /> New payment
        </ButtonLink>
      </div>
    </Card>
  );
}

function ClosedView({ payment, icon, title, text }: { payment: Payment; icon: ReactNode; title: string; text: string }) {
  return (
    <Card className="mx-auto max-w-xl p-8 text-center">
      <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-stone-100 text-stone-600">{icon}</div>
      <p className="mt-4 font-heading text-xl font-bold">{title}</p>
      <p className="tabular mt-1 text-muted">
        {formatNumber(payment.amount_sats)} sats{payment.description ? ` · ${payment.description}` : ""}
      </p>
      <p className="mt-3 text-sm text-muted">{text}</p>
      <div className="mt-6">
        <ButtonLink href="/receive" size="lg">
          <QrCode aria-hidden className="h-5 w-5" /> Create a new request
        </ButtonLink>
      </div>
    </Card>
  );
}

function Row({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 px-6 py-3.5 text-sm">
      <dt className="text-muted">{label}</dt>
      <dd className="text-right font-semibold">{value}</dd>
    </div>
  );
}

function useSecondsUntil(iso: string) {
  const target = new Date(iso).getTime();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  return Math.max(0, Math.round((target - now) / 1000));
}

function formatCountdown(s: number) {
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, "0")}`;
}
