const base = "rounded border px-3 py-1 text-sm disabled:cursor-not-allowed disabled:opacity-40";
const small = "rounded border px-2 py-0.5 text-xs disabled:cursor-not-allowed disabled:opacity-40";
const plain =
  "border-zinc-300 bg-white hover:bg-zinc-100 dark:border-zinc-600 dark:bg-zinc-800 dark:hover:bg-zinc-700";
export const btn = `${base} ${plain}`;
export const btnSmall = `${small} ${plain}`;
export const primary = `${base} border-green-700 bg-green-600 text-white hover:bg-green-700`;
export const danger = `${base} border-red-700 bg-red-600 text-white hover:bg-red-700`;
