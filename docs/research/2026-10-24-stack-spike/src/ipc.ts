// Shared IPC contract: imported by main, preload and renderer (types only in renderer).
export const IPC = {
  getVersions: 'app:get-versions',
  tick: 'app:tick',
} as const;

export interface Api {
  getVersions(): Promise<NodeJS.ProcessVersions>;
  /** Returns an unsubscribe function. */
  onTick(cb: (count: number) => void): () => void;
}
