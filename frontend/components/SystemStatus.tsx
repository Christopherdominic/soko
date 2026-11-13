"use client";

import { useEffect, useState } from "react";
import type { HealthResponse } from "@/types/api";

type State = { kind: "loading" } | { kind: "loaded"; health: HealthResponse } | { kind: "error" };

/** Compact connection status: server, records database, Lightning node. */
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
    const id = setInterval(load, 15_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const rows: { label: string; ok: boolean | null; text: string }[] =
    state.kind === "loading"
      ? [{ label: "Checking connection", ok: null, text: "…" }]
      : state.kind === "error"
        ? [{ label: "Soko server", ok: false, text: "Not reachable" }]
        : [
            { label: "Soko server", ok: true, text: "Online" },
            { label: "Records", ok: state.health.database === "ok", text: state.health.database === "ok" ? "Online" : "Offline" },
            {
              label: "Lightning node",
              ok: state.health.lightning === "ok",
              text:
                state.health.lightning === "ok"
                  ? "Ready to receive"
                  : state.health.lightning === "starting"
                    ? "Starting up"
                    : "Offline",
            },
          ];

  return (
    <ul className="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-3" aria-live="polite">
      {rows.map((r) => (
        <li key={r.label} className="flex items-center justify-between gap-3 sm:justify-start">
          <span className="text-muted">{r.label}</span>
          <span className="flex items-center gap-1.5 font-medium">
            <span
              aria-hidden
              className={`h-2 w-2 rounded-full ${r.ok === null ? "bg-stone-300" : r.ok ? "bg-emerald-500" : "bg-amber-500"}`}
            />
            {r.text}
          </span>
        </li>
      ))}
    </ul>
  );
}
