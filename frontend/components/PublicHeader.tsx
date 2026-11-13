import Link from "next/link";
import { Logo } from "@/components/Logo";

export function PublicHeader() {
  return (
    <header className="border-b border-line bg-surface">
      <div className="mx-auto flex max-w-xl items-center justify-between px-4 py-3.5">
        <Link href="/login" aria-label="Soko">
          <Logo />
        </Link>
        <span className="text-sm text-muted">Every market. Every payment.</span>
      </div>
    </header>
  );
}

export function PublicFooter() {
  return (
    <p className="pb-10 text-center text-xs text-subtle">
      Receipts on Soko are digitally signed and can be verified by anyone.
    </p>
  );
}
