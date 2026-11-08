"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";
import QRCode from "qrcode";
import { api, ApiError } from "@/lib/api";
import { formatDateTime, formatSats } from "@/lib/format";
import { StatusBadge } from "@/components/StatusBadge";
import { PrimaryLink, SecondaryButton } from "@/components/ui";
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
    return <p className="p-6 text-center text-muted">{error ?? "Loading…"}</p>;
  }

  return (
    <main className="mx-auto w-full max-w-md flex-1 px-4 py-6">
      <Link href="/" className="text-sm text-muted underline-offset-4 hover:underline">
        ← Home
      </Link>
      <div className="mt-3 mb-5 flex items-start justify-between gap-3">
        <div>
          <p className="text-3xl font-extrabold">{formatSats(payment.amount_sats)}</p>
          {payment.description && <p className="text-muted">{payment.description}</p>}
        </div>
        <StatusBadge status={payment.status} />
      </div>

      {payment.status === "pending" && <PendingView payment={payment} />}
      {payment.status === "paid" && <PaidView payment={payment} />}
      {payment.status === "expired" && (
        <ClosedView title="This payment request expired" text="No money was received. Create a new request to try again." />
      )}
      {payment.status === "failed" && (
        <ClosedView title="This payment could not be completed" text="No receipt was issued. Create a new request to try again." />
      )}
      {error && <p className="mt-4 text-center text-sm text-amber-800">{error}</p>}
    </main>
  );
}

function PendingView({ payment }: { payment: Payment }) {
  const [qr, setQr] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const secondsLeft = useSecondsUntil(payment.expires_at);

  useEffect(() => {
    // Uppercase BOLT11 fits QR alphanumeric mode, giving a smaller, easier-to-scan code.
    QRCode.toDataURL(`lightning:${payment.payment_request}`.toUpperCase(), {
      margin: 1,
      width: 640,
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
    <div className="space-y-4">
      <div className="rounded-3xl border border-line bg-white p-4">
        {qr ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={qr} alt="Lightning payment QR code" className="mx-auto aspect-square w-full max-w-sm" />
        ) : (
          <div className="aspect-square w-full" />
        )}
      </div>
      <p className="text-center font-semibold" aria-live="polite">
        Waiting for payment…
        <span className="block text-sm font-normal text-muted">
          Ask the customer to scan with a Lightning wallet. Expires in {formatCountdown(secondsLeft)}.
        </span>
      </p>
      <SecondaryButton onClick={copy}>{copied ? "Copied ✓" : "Copy payment request"}</SecondaryButton>
    </div>
  );
}

function PaidView({ payment }: { payment: Payment }) {
  return (
    <div className="space-y-5">
      <div className="rounded-3xl bg-green-50 px-6 py-8 text-center" role="status">
        <div aria-hidden className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-green-600 text-3xl text-white">
          ✓
        </div>
        <p className="mt-4 text-2xl font-extrabold text-green-900">Payment received</p>
        <p className="text-green-900/80">Confirmed by your Lightning node</p>
      </div>
      <dl className="divide-y divide-line rounded-2xl border border-line bg-white">
        <Row label="Receipt" value={payment.receipt_number ?? "Issuing…"} />
        <Row label="Amount" value={formatSats(payment.amount_sats)} />
        {payment.settled_at && <Row label="Paid at" value={formatDateTime(payment.settled_at)} />}
      </dl>
      <div className="grid gap-3">
        <PrimaryLink href="/receive">Receive another payment</PrimaryLink>
        <Link
          href={`/payments/${payment.payment_hash}/receipt`}
          className="flex h-12 items-center justify-center rounded-2xl border-2 border-forest font-semibold text-forest"
        >
          View &amp; share receipt
        </Link>
      </div>
    </div>
  );
}

function ClosedView({ title, text }: { title: string; text: string }) {
  return (
    <div className="space-y-5">
      <div className="rounded-3xl bg-stone-100 px-6 py-8 text-center">
        <p className="text-xl font-bold">{title}</p>
        <p className="mt-1 text-muted">{text}</p>
      </div>
      <PrimaryLink href="/receive">Create a new request</PrimaryLink>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4 px-4 py-3">
      <dt className="text-muted">{label}</dt>
      <dd className="font-semibold">{value}</dd>
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
