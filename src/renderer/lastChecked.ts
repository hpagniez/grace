const rtf = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

/** `Last checked 2 minutes ago` for the ISO time of the last successful check; "" before the first (R2.6). */
export function lastCheckedText(iso: string | null, now: number): string {
  const then = iso === null ? NaN : Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const minutes = Math.max(0, Math.floor((now - then) / 60_000));
  const ago =
    minutes < 60
      ? rtf.format(-minutes, "minute")
      : minutes < 1440
        ? rtf.format(-Math.floor(minutes / 60), "hour")
        : rtf.format(-Math.floor(minutes / 1440), "day");
  return `Last checked ${ago}`;
}
