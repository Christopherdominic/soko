import Link from "next/link";
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";
import { AlertCircle, Loader2 } from "lucide-react";

type Variant = "primary" | "secondary" | "ghost";
type Size = "md" | "lg";

const base =
  "inline-flex items-center justify-center gap-2 rounded-xl font-semibold transition-colors focus-visible:outline-3 focus-visible:outline-offset-2 focus-visible:outline-lime disabled:cursor-not-allowed disabled:opacity-50";
const variants: Record<Variant, string> = {
  primary: "bg-forest text-ivory hover:bg-forest-800",
  secondary: "border border-line-strong bg-surface text-forest hover:bg-ivory",
  ghost: "text-muted hover:bg-black/5 hover:text-charcoal",
};
const sizes: Record<Size, string> = {
  md: "h-11 px-4 text-sm",
  lg: "h-14 px-6 text-base",
};

export function buttonClass(variant: Variant = "primary", size: Size = "md", full = false) {
  return `${base} ${variants[variant]} ${sizes[size]} ${full ? "w-full" : ""}`;
}

export function Button({
  variant = "primary",
  size = "md",
  full = false,
  loading = false,
  className = "",
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: Size; full?: boolean; loading?: boolean }) {
  return (
    <button {...props} disabled={props.disabled || loading} className={`${buttonClass(variant, size, full)} ${className}`}>
      {loading && <Loader2 aria-hidden className="h-4 w-4 animate-spin" />}
      {children}
    </button>
  );
}

export function ButtonLink({
  href,
  variant = "primary",
  size = "md",
  full = false,
  children,
}: {
  href: string;
  variant?: Variant;
  size?: Size;
  full?: boolean;
  children: ReactNode;
}) {
  return (
    <Link href={href} className={buttonClass(variant, size, full)}>
      {children}
    </Link>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <section className={`rounded-2xl border border-line bg-surface shadow-[var(--shadow-card)] ${className}`}>{children}</section>;
}

export function CardHeader({ title, action, subtitle }: { title: string; subtitle?: string; action?: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
      <div>
        <h2 className="text-base font-bold">{title}</h2>
        {subtitle && <p className="mt-0.5 text-sm text-muted">{subtitle}</p>}
      </div>
      {action}
    </div>
  );
}

export function PageHeader({ title, subtitle, action }: { title: string; subtitle?: ReactNode; action?: ReactNode }) {
  return (
    <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 className="text-2xl font-extrabold text-forest sm:text-3xl">{title}</h1>
        {subtitle && <p className="mt-1 text-muted">{subtitle}</p>}
      </div>
      {action}
    </div>
  );
}

export function Field({
  label,
  hint,
  suffix,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & { label: string; hint?: string; suffix?: string }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm font-semibold text-charcoal">{label}</span>
      <span className="relative block">
        <input
          {...props}
          className={`h-12 w-full rounded-xl border border-line-strong bg-surface px-4 text-base outline-none transition-shadow placeholder:text-subtle focus:border-forest focus:ring-4 focus:ring-forest/10 ${suffix ? "pr-14" : ""}`}
        />
        {suffix && (
          <span className="pointer-events-none absolute inset-y-0 right-4 flex items-center text-sm font-medium text-muted">
            {suffix}
          </span>
        )}
      </span>
      {hint && <span className="mt-1.5 block text-sm text-muted">{hint}</span>}
    </label>
  );
}

export function ErrorMessage({ children }: { children: ReactNode }) {
  if (!children) return null;
  return (
    <p role="alert" className="flex items-start gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertCircle aria-hidden className="mt-0.5 h-4 w-4 shrink-0" />
      <span>{children}</span>
    </p>
  );
}

export function EmptyState({ icon, title, text, action }: { icon: ReactNode; title: string; text?: string; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center px-6 py-12 text-center">
      <div className="flex h-12 w-12 items-center justify-center rounded-full bg-ivory text-forest">{icon}</div>
      <p className="mt-4 font-semibold">{title}</p>
      {text && <p className="mt-1 max-w-sm text-sm text-muted">{text}</p>}
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}

export function Loading({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 py-16 text-muted">
      <Loader2 aria-hidden className="h-5 w-5 animate-spin" />
      <span>{label}</span>
    </div>
  );
}
