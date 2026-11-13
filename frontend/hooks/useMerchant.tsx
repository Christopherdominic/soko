"use client";

import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { useRouter } from "next/navigation";
import { api, ApiError } from "@/lib/api";
import { Loading } from "@/components/ui";
import type { MerchantWithKey as Merchant } from "@/types/api";

const MerchantContext = createContext<Merchant | null>(null);

export function useMerchant(): Merchant {
  const m = useContext(MerchantContext);
  if (!m) throw new Error("useMerchant must be used inside <RequireMerchant>");
  return m;
}

/** Renders children only for a logged-in merchant; otherwise sends them to /login. */
export function RequireMerchant({ children }: { children: ReactNode }) {
  const router = useRouter();
  const [merchant, setMerchant] = useState<Merchant | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .me()
      .then(setMerchant)
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) router.replace("/login");
        else setError(e instanceof ApiError ? e.message : "Something went wrong");
      });
  }, [router]);

  if (error) return <p className="p-6 text-center text-red-700">{error}</p>;
  if (!merchant) return <Loading />;
  return <MerchantContext.Provider value={merchant}>{children}</MerchantContext.Provider>;
}
