import Link from "next/link";
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";

export function PrimaryButton({ className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      {...props}
      className={`h-14 w-full rounded-2xl bg-forest text-lg font-bold text-ivory transition-opacity disabled:opacity-50 focus-visible:outline-4 focus-visible:outline-lime ${className}`}
    />
  );
}

export function PrimaryLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <Link
      href={href}
      className="flex h-14 w-full items-center justify-center rounded-2xl bg-forest text-lg font-bold text-ivory focus-visible:outline-4 focus-visible:outline-lime"
    >
      {children}
    </Link>
  );
}

export function SecondaryButton({ className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      {...props}
      className={`h-12 w-full rounded-2xl border-2 border-forest font-semibold text-forest disabled:opacity-50 ${className}`}
    />
  );
}

export function Field({
  label,
  hint,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & { label: string; hint?: string }) {
  return (
    <label className="block">
      <span className="mb-1.5 block font-semibold">{label}</span>
      <input
        {...props}
        className="h-14 w-full rounded-xl border-2 border-line bg-white px-4 text-lg outline-none focus:border-forest"
      />
      {hint && <span className="mt-1 block text-sm text-muted">{hint}</span>}
    </label>
  );
}

export function ErrorMessage({ children }: { children: ReactNode }) {
  if (!children) return null;
  return (
    <p role="alert" className="rounded-xl bg-red-50 px-4 py-3 text-red-800">
      {children}
    </p>
  );
}
