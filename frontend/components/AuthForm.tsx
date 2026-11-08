"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { api, ApiError } from "@/lib/api";
import { ErrorMessage, Field, PrimaryButton } from "@/components/ui";

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
    <main className="mx-auto w-full max-w-md flex-1 px-4 py-10">
      <div className="mb-8">
        <p className="font-heading text-2xl font-extrabold text-forest">Soko</p>
        <h1 className="mt-4 text-3xl font-extrabold">
          {mode === "login" ? "Welcome back" : "Set up your business"}
        </h1>
        <p className="mt-1 text-muted">
          {mode === "login" ? "Log in to receive payments." : "Start accepting Lightning payments in minutes."}
        </p>
      </div>
      <form onSubmit={onSubmit} className="space-y-5">
        {mode === "register" && (
          <Field label="Business name" name="business_name" required maxLength={120} autoComplete="organization" />
        )}
        <Field label="Email" name="email" type="email" required autoComplete="email" />
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
        <PrimaryButton type="submit" disabled={busy}>
          {busy ? "Please wait…" : mode === "login" ? "Log in" : "Create account"}
        </PrimaryButton>
      </form>
      <p className="mt-6 text-center text-muted">
        {mode === "login" ? (
          <>New to Soko? <Link href="/register" className="font-semibold text-forest underline">Create an account</Link></>
        ) : (
          <>Already have an account? <Link href="/login" className="font-semibold text-forest underline">Log in</Link></>
        )}
      </p>
    </main>
  );
}
