"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import type { ReactNode } from "react";
import { LayoutDashboard, LogOut, QrCode, ReceiptText } from "lucide-react";
import { RequireMerchant, useMerchant } from "@/hooks/useMerchant";
import { Logo, LogoMark } from "@/components/Logo";
import { api } from "@/lib/api";

const NAV = [
  { href: "/", label: "Dashboard", Icon: LayoutDashboard },
  { href: "/receive", label: "Receive", Icon: QrCode },
  { href: "/history", label: "Transactions", Icon: ReceiptText },
];

function isActive(pathname: string, href: string) {
  if (href === "/") return pathname === "/";
  if (href === "/history") return pathname.startsWith("/history") || pathname.startsWith("/payments");
  return pathname.startsWith(href);
}

/** Authenticated layout: sidebar on desktop, top bar + bottom tabs on phones. */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <RequireMerchant>
      <Shell>{children}</Shell>
    </RequireMerchant>
  );
}

function Shell({ children }: { children: ReactNode }) {
  const merchant = useMerchant();
  const pathname = usePathname();
  const router = useRouter();
  const initials = merchant.business_name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join("");

  const logout = async () => {
    await api.logout().catch(() => {});
    router.replace("/login");
  };

  return (
    <div className="min-h-full flex-1">
      {/* Desktop sidebar */}
      <aside className="fixed inset-y-0 left-0 hidden w-64 flex-col bg-forest text-ivory lg:flex">
        <div className="px-6 pt-7 pb-8">
          <Link href="/" aria-label="Soko dashboard">
            <Logo tone="light" />
          </Link>
        </div>
        <nav className="flex-1 space-y-1 px-3" aria-label="Main">
          {NAV.map(({ href, label, Icon }) => {
            const active = isActive(pathname, href);
            return (
              <Link
                key={href}
                href={href}
                aria-current={active ? "page" : undefined}
                className={`flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-semibold transition-colors ${
                  active ? "bg-lime text-forest" : "text-ivory/75 hover:bg-white/5 hover:text-ivory"
                }`}
              >
                <Icon aria-hidden className="h-5 w-5" />
                {label}
              </Link>
            );
          })}
        </nav>
        <div className="m-3 rounded-xl bg-white/5 p-3">
          <div className="flex items-center gap-3">
            <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-forest-600 text-sm font-bold">
              {initials}
            </span>
            <div className="min-w-0">
              <p className="truncate text-sm font-semibold">{merchant.business_name}</p>
              <p className="truncate text-xs text-ivory/60">{merchant.email}</p>
            </div>
          </div>
          <button
            onClick={logout}
            className="mt-3 flex w-full items-center justify-center gap-2 rounded-lg py-2 text-sm text-ivory/75 hover:bg-white/5 hover:text-ivory"
          >
            <LogOut aria-hidden className="h-4 w-4" /> Log out
          </button>
        </div>
      </aside>

      {/* Mobile top bar */}
      <header className="sticky top-0 z-20 flex items-center justify-between border-b border-line bg-ivory/90 px-4 py-3 backdrop-blur lg:hidden">
        <Link href="/" className="flex min-w-0 items-center gap-2.5" aria-label="Soko dashboard">
          <LogoMark className="h-8 w-8 shrink-0" />
          <span className="truncate font-heading font-bold text-forest">{merchant.business_name}</span>
        </Link>
        <button onClick={logout} aria-label="Log out" className="rounded-lg p-2 text-muted hover:bg-black/5">
          <LogOut aria-hidden className="h-5 w-5" />
        </button>
      </header>

      <main className="px-4 pt-5 pb-28 sm:px-6 lg:ml-64 lg:px-10 lg:pt-10 lg:pb-12">
        <div className="mx-auto w-full max-w-5xl">{children}</div>
      </main>

      {/* Mobile bottom tabs */}
      <nav
        aria-label="Main"
        className="fixed inset-x-0 bottom-0 z-20 border-t border-line bg-surface/95 pb-[env(safe-area-inset-bottom)] backdrop-blur lg:hidden"
      >
        <div className="mx-auto grid max-w-md grid-cols-3 items-end">
          {NAV.map(({ href, label, Icon }) => {
            const active = isActive(pathname, href);
            if (href === "/receive") {
              return (
                <Link key={href} href={href} aria-current={active ? "page" : undefined} className="flex flex-col items-center pb-2">
                  <span className="-mt-6 flex h-14 w-14 items-center justify-center rounded-2xl bg-forest text-lime shadow-[var(--shadow-raised)]">
                    <Icon aria-hidden className="h-6 w-6" />
                  </span>
                  <span className={`mt-1 text-xs font-semibold ${active ? "text-forest" : "text-muted"}`}>{label}</span>
                </Link>
              );
            }
            return (
              <Link
                key={href}
                href={href}
                aria-current={active ? "page" : undefined}
                className={`flex flex-col items-center gap-1 py-2.5 text-xs font-semibold ${active ? "text-forest" : "text-muted"}`}
              >
                <Icon aria-hidden className="h-5 w-5" />
                {label === "Transactions" ? "History" : label}
              </Link>
            );
          })}
        </div>
      </nav>
    </div>
  );
}
