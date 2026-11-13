/** Soko mark: a market-stall awning over a lightning bolt. */
export function LogoMark({ className = "h-9 w-9" }: { className?: string }) {
  return (
    <svg viewBox="0 0 40 40" className={className} aria-hidden>
      <rect width="40" height="40" rx="11" fill="#D9F27E" />
      {/* Striped awning with a scalloped edge */}
      <path d="M8 9.5A1.5 1.5 0 0 1 9.5 8h21A1.5 1.5 0 0 1 32 9.5V14H8Z" fill="#142D27" />
      <path d="M8 14h6a3 3 0 0 1-6 0Zm12 0h6a3 3 0 0 1-6 0Z" fill="#142D27" />
      <path d="M14 14h6a3 3 0 0 1-6 0Zm12 0h6a3 3 0 0 1-6 0Z" fill="#142D27" opacity=".55" />
      <path d="M14 8h6v6h-6Zm12 0h4.5A1.5 1.5 0 0 1 32 9.5V14h-6Z" fill="#D9F27E" opacity=".35" />
      {/* Lightning bolt */}
      <path d="M22.2 19.5 15.5 28h4.6L18.4 34.5l7-8.7h-4.7l1.5-6.3Z" fill="#142D27" />
    </svg>
  );
}

export function Logo({ tone = "dark" }: { tone?: "dark" | "light" }) {
  return (
    <span className="flex items-center gap-2.5">
      <LogoMark />
      <span className={`font-heading text-xl font-extrabold tracking-tight ${tone === "light" ? "text-ivory" : "text-forest"}`}>
        Soko
      </span>
    </span>
  );
}
