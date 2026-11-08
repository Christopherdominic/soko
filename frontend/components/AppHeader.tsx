"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useMerchant } from "@/hooks/useMerchant";
import { api } from "@/lib/api";

export function AppHeader() {
  const merchant = useMerchant();
  const router = useRouter();

  const logout = async () => {
    await api.logout().catch(() => {});
    router.replace("/login");
  };

  return (
    <header className="bg-forest text-ivory">
      <div className="mx-auto flex max-w-2xl items-center justify-between px-4 py-4">
        <Link href="/" className="flex flex-col">
          <span className="font-heading text-xl font-extrabold text-lime">Soko</span>
          <span className="text-sm text-ivory/80">{merchant.business_name}</span>
        </Link>
        <button onClick={logout} className="rounded-lg px-3 py-2 text-sm text-ivory/90 underline-offset-4 hover:underline">
          Log out
        </button>
      </div>
    </header>
  );
}
