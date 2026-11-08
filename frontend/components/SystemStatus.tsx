"use client";

import { useEffect, useState } from "react";
import type { HealthResponse } from "@/types/api";

type State = { kind: "loading" } | { kind: "loaded"; health: HealthResponse } | { kind: "error" };

export function SystemStatus() {
  const [state, setState] = useState<State>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const res = await fetch("/api/health", { cache: "no-store" });
        const health = (await res.json()) as HealthResponse;
        if (!cancelled) setState({ kind: "loaded", health });
      } catch {
        if (!cancelled) setState({ kind: "error" });
      }
    };
    load();
    const id = setInterval(load, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (state.kind === "loading") {
    return <p className="text-muted">Checking connection…</p>;
  }
  if (state.kind === "error") {
    return <Row label="Soko server" ok={false} text="Not reachable" />;
  }
  const { health } = state;
  return (
    <ul className="space-y-3">
      <Row label="Soko server" ok text="Online" />
      <Row label="Records database" ok={health.database === "ok"} text={health.database === "ok" ? "Online" : "Offline"} />
      <Row
        label="Lightning node"
        ok={health.lightning === "ok"}
        text={health.lightning === "ok" ? "Ready to receive" : health.lightning === "starting" ? "Starting up" : "Offline"}
      />
    </ul>
  );
}

function Row({ label, ok, text }: { label: string; ok: boolean; text: string }) {
  return (
    <li className="flex items-center justify-between rounded-xl bg-white px-4 py-3 border border-line">
      <span className="font-medium">{label}</span>
      <span className="flex items-center gap-2 text-sm">
        <span aria-hidden className={`h-2.5 w-2.5 rounded-full ${ok ? "bg-green-600" : "bg-amber-500"}`} />
        {text}
      </span>
    </li>
  );
}
