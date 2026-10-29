const base =
  "inline-flex items-center justify-center gap-1.5 rounded-md whitespace-nowrap transition-colors duration-(--duration-fast) ease-standard disabled:cursor-not-allowed disabled:opacity-40";
const md = "h-7 px-3 text-sm"; // 28 px (spec R3.1)
const sm = "h-6 px-2 text-xs"; // 24 px
const plain = "border border-control bg-surface text-ink enabled:hover:bg-hover";
const quiet = "text-ink enabled:hover:bg-hover";

export const btn = `${base} ${md} ${plain}`;
export const btnSmall = `${base} ${sm} ${plain}`;
export const primary = `${base} ${md} bg-accent font-semibold text-on-accent enabled:hover:bg-accent/85`;
export const danger = `${base} ${md} bg-failed text-on-danger enabled:hover:bg-failed/85`;
/** No border, so a row of them stays quiet. */
export const ghost = `${base} ${md} ${quiet}`;
export const ghostSmall = `${base} ${sm} ${quiet}`;
/** Icon only: 28 × 28, borderless. Give it an aria-label. */
export const btnIcon = `${base} size-7 ${quiet}`;

/**
 * The toolbar below 1150 px: a button's label is only spoken (put narrowLabel on a span around it)
 * and the button shrinks to its icon, so the toolbar fits down to the window's 760 px minimum.
 */
export const narrowIcon = "max-[1150px]:w-7 max-[1150px]:px-0";
export const narrowLabel = "max-[1150px]:sr-only";
/** The width of the two strings above, for a script that must know (a tooltip only while the label is hidden). */
export const NARROW_QUERY = "(width < 1150px)"; // what Tailwind's max-[1150px] means
