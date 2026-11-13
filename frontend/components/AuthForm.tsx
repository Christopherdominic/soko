"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState, type FormEvent, type ReactNode } from "react";
import { ReceiptText, ShieldCheck, Zap } from "lucide-react";
import { api, ApiError } from "@/lib/api";
import { Logo } from "@/components/Logo";
import { Button, ErrorMessage, Field } from "@/components/ui";

export function AuthForm({ mode }: { mode: "login" | "register" }) {
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const form = new FormData(e.currentTarget);
    const email = String(form.get("email") ?? "");
    const password = String(form.get("password") ?? "");
    setBusy(true);
    setError(null);
    try {
      if (mode === "register") {
        await api.register(String(form.get("business_name") ?? ""), email, password);
      } else {
        await api.login(email, password);
      }
      router.replace("/");
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong");
      setBusy(false);
    }
  };

  return (
    <div className="grid min-h-full flex-1 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
      {/* Brand panel (desktop) */}
      <aside className="relative hidden overflow-hidden bg-forest p-12 text-ivory lg:flex lg:flex-col">
        <Logo tone="light" />
        <div className="my-auto max-w-md">
          <h2 className="font-heading text-4xl leading-tight font-extrabold">
            Every market.
            <br />
            <span className="text-lime">Every payment.</span>
          </h2>
          <p className="mt-4 text-lg text-ivory/75">Smarter payments for everyday business.</p>
          <ul className="mt-10 space-y-5">
            <Point icon={<Zap aria-hidden className="h-4 w-4" />} title="Paid in seconds" text="Customers pay with any Lightning wallet." />
            <Point icon={<ShieldCheck aria-hidden className="h-4 w-4" />} title="Verified by your own node" text="No middleman holds your money." />
            <Point icon={<ReceiptText aria-hidden className="h-4 w-4" />} title="Receipts and records" text="Signed receipts and a clear sales history." />
          </ul>
        </div>
        <p className="text-sm text-ivory/50">Bitcoin payments for small businesses.</p>
        <div aria-hidden className="absolute -right-24 -bottom-24 h-72 w-72 rounded-full bg-lime/10" />
      </aside>

      {/* Form */}
      <main className="flex items-center justify-center px-4 py-10 sm:px-8">
        <div className="w-full max-w-sm">
          <div className="mb-10 lg:hidden">
            <Logo />
          </div>
          <h1 className="text-3xl font-extrabold text-forest">{mode === "login" ? "Welcome back" : "Set up your business"}</h1>
          <p className="mt-2 text-muted">
            {mode === "login" ? "Log in to receive payments and see your sales." : "Start accepting Lightning payments in minutes."}
          </p>
          <form onSubmit={onSubmit} className="mt-8 space-y-5">
            {mode === "register" && (
              <Field label="Business name" name="business_name" required maxLength={120} autoComplete="organization" placeholder="e.g. Mama Amina's Market Stall" />
            )}
            <Field label="Email" name="email" type="email" required autoComplete="email" placeholder="you@example.com" />
            <Field
              label="Password"
              name="password"
              type="password"
              required
              minLength={8}
              maxLength={128}
              autoComplete={mode === "login" ? "current-password" : "new-password"}
              hint={mode === "register" ? "At least 8 characters" : undefined}
            />
            <ErrorMessage>{error}</ErrorMessage>
            <Button type="submit" size="lg" full loading={busy}>
              {busy ? "Please wait…" : mode === "login" ? "Log in" : "Create account"}
            </Button>
          </form>
          <p className="mt-8 text-center text-sm text-muted">
            {mode === "login" ? (
              <>
                New to Soko?{" "}
                <Link href="/register" className="font-semibold text-forest hover:underline">
                  Create an account
                </Link>
              </>
            ) : (
              <>
                Already have an account?{" "}
                <Link href="/login" className="font-semibold text-forest hover:underline">
                  Log in
                </Link>
              </>
            )}
          </p>
        </div>
      </main>
    </div>
  );
}

function Point({ icon, title, text }: { icon: ReactNode; title: string; text: string }) {
  return (
    <li className="flex gap-3">
      <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-lime text-forest">{icon}</span>
      <span>
        <span className="block font-semibold">{title}</span>
        <span className="block text-sm text-ivory/70">{text}</span>
      </span>
    </li>
  );
}
