export const formatSats = (n: number) => `${n.toLocaleString("en")} sats`;

export const formatDateTime = (iso: string) =>
  new Date(iso).toLocaleString(undefined, {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
