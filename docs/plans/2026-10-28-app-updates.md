# App Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Grace shows its version, checks GitHub Releases for newer versions on a stable or pre-release channel, and installs any published version in place, newer or older. It backs up the data before every switch and switches back if the new version does not start.

**Architecture:** Pure version and release logic lives in `src/shared/` and runs in both processes. The main process (`src/main/updates/`) checks the releases API, downloads, verifies and unpacks a release next to the install folder, then hands over to a PowerShell switch script and quits. The script backs up and restores data, swaps the folders, waits for the new version's "started" signal, and writes a result file that the next Grace reads. The renderer adds a status bar, a startup modal, a progress dialog and a Settings page, all driven by `updates`, `updatePrompt` and `updateProgress` events from an `Updater` in main.

**Tech Stack:** Electron 45, SolidJS 2 RC, Tailwind v4, Vitest (node and jsdom projects), Playwright `_electron`, Node `fetch`, `crypto` and `fs`, Windows PowerShell 5.1, `%SystemRoot%\System32\tar.exe`.

**Spec:** [docs/specs/2026-10-28-app-updates.md](../specs/2026-10-28-app-updates.md) (requirements R1–R13), with [ADR 0019](../decisions/0019-update-the-zip-install-from-github-releases.md) and the [mockup](../specs/2026-10-28-app-updates-mockup.png). Read the spec before any task: the plan cites its requirement numbers instead of repeating them.

## Global Constraints

- No new dependency. Pin nothing new.
- `contextIsolation`, `sandbox`, `nodeIntegration: false` and the CSP `default-src 'self'` stay. Every network request, file operation and `shell.openExternal` call runs in main. The renderer reaches them only through new typed preload calls, and main validates every argument (version strings against `parseVersion`, booleans, the channel).
- Solid 2 rules from AGENTS.md (`@solidjs/web`, `createEffect` with compute and apply functions, `onSettled`, store setters take a draft, no `classList`).
- Windows only. Run tools by absolute path: `%SystemRoot%\System32\tar.exe` and `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe`, never from `PATH` (Git's GNU tar cannot read zip).
- Constants: `FIRST_UPDATER = "1.0.0-rc.4"`. The releases API is `https://api.github.com/repos/hpagniez/grace/releases?per_page=100`, with a 15 s timeout. The start wait is 120 s and the exit wait 60 s. Keep 3 backups. The disk check asks for 4 × the zip size. Sizes in MB are `Math.round(bytes / 1e6)`.
- User-facing copy is verbatim from the spec (R3–R10). The strings in the tests below are copied from it; do not reword them.
- Files the switch script writes are UTF-8 without a byte order mark.
- Buttons use `btn`/`primary` from `src/renderer/buttons.ts`. Dialogs follow `PropertiesDialog.tsx`: a `<dialog>` with `showModal()`, `aria-labelledby`, `backdrop:bg-black/40`.
- Never `npm test` and `npm run e2e` at the same time (AGENTS.md).

## Review Focus

1. An install folder whose path has spaces and non-ASCII letters (`C:\Users\Zoë\My Tools\Grace`) must survive the plan JSON, PowerShell and `tar.exe`. Tested in Tasks 5 and 6.
2. A zip entry with `..` or an absolute path must not write outside the staging folder. Tested in Task 5.
3. GitHub answering 403 (rate limit), HTML, or a JSON object instead of an array must give a quiet failure with a reason, never a throw out of the startup path. Tested in Task 3.
4. Grace quit or killed during a download leaves `<install>.update` with a partial zip. The next start removes it and never hands it over. Tested in Task 8.
5. Version strings with leading zeros or extra parts (`1.0.0-rc.04`, `1.0.0-rc.4.1`, `v1.0.0`, `1.0`) must be ignored, never ranked. Tested in Task 1.

---

### Task 1: Versions and releases

**Files:**
- Create: `src/shared/version.ts`, `src/shared/updates.ts`
- Test: `src/shared/version.test.ts`, `src/shared/updates.test.ts`

**Interfaces:**
- Produces:
  - `parseVersion(s: string): { major: number; minor: number; patch: number; rc: number | null } | null`
  - `compareVersions(a: string, b: string): number` (negative, 0 or positive; both must be valid, otherwise it throws)
  - `isPrerelease(v: string): boolean`
  - In `updates.ts`: `type Channel = "stable" | "prerelease"`, `FIRST_UPDATER`, `RELEASES_API`, `releasePage(v: string): string`, `zipName(v: string): string`, `downloadPrefix(v: string): string` (`https://github.com/hpagniez/grace/releases/download/v<v>/`)
  - `interface Release { version: string; prerelease: boolean; publishedAt: string; url: string; size: number; sha256: string }`
  - `parseReleases(json: unknown): Release[]` (R1.4, newest first, one per version)
  - `onChannel(rs: Release[], c: Channel): Release[]` and `latestOn(rs: Release[], c: Channel): Release | null`
  - `interface UpdateSettings { channel: Channel; checkAtStartup: boolean; autoUpdate: boolean; skipped: string | null; failed: string | null; lastChecked: string | null }`
  - `defaultUpdateSettings(running: string): UpdateSettings` (R1.3, R12)
  - `promptFor(rs: Release[], s: UpdateSettings, running: string): Release | null` (R4.1)
  - `autoUpdateTo(rs: Release[], s: UpdateSettings, running: string): Release | null` (R5.2)
  - `interface Backup { version: string; savedAt: string }`
  - `interface UpdateView { running: string; packaged: boolean; settings: UpdateSettings; releases: Release[] | null; checking: boolean; error: string | null; backups: Backup[] }`
  - `type UpdateProgress = { version: string; rollback: boolean; step: "download" | "check" | "unpack"; received: number; total: number } | { version: string; rollback: boolean; error: string }`

- [ ] **Step 1: Write the failing tests**

`version.test.ts`:
```ts
it("parses only X.Y.Z and X.Y.Z-rc.N", () => {
  expect(parseVersion("1.0.0-rc.4")).toEqual({ major: 1, minor: 0, patch: 0, rc: 4 });
  expect(parseVersion("1.2.3")).toEqual({ major: 1, minor: 2, patch: 3, rc: null });
  for (const bad of ["v1.0.0", "1.0", "1.0.0-rc.04", "01.0.0", "1.0.0-rc.4.1", "1.0.0-beta.1", "1.0.0-rc", ""])
    expect(parseVersion(bad)).toBeNull();
});
it("orders by SemVer precedence", () => {
  const sorted = ["1.0.0", "1.0.0-rc.10", "0.9.9", "1.0.0-rc.9", "1.0.1-rc.1"].sort(compareVersions);
  expect(sorted).toEqual(["0.9.9", "1.0.0-rc.9", "1.0.0-rc.10", "1.0.0", "1.0.1-rc.1"]);
});
```
`updates.test.ts` builds GitHub-shaped entries with a helper `gh(tag, { prerelease, draft, assets })`, where each asset is `{ name, size, digest, browser_download_url }`:
```ts
it("keeps installable releases only (R1.4)", () => {
  const rs = parseReleases([
    gh("v1.0.0-rc.5"), gh("v1.0.0-rc.3"), gh("v1.0.0-rc.6", { draft: true }), gh("v2.0.0-beta"),
    gh("v1.0.0-rc.7", { assets: [] }), gh("v1.0.0-rc.8", { digest: "md5:abc" }),
    gh("v1.0.0-rc.9", { url: "https://evil.example/Grace-win32-x64-1.0.0-rc.9.zip" }),
  ]);
  expect(rs.map((r) => r.version)).toEqual(["1.0.0-rc.5"]); // rc.3 is below FIRST_UPDATER
});
it("ignores anything that is not an array of objects", () => {
  for (const x of [null, {}, "<html>", [1, "a", null]]) expect(parseReleases(x)).toEqual([]);
});
it("stable channel hides pre-releases", () => {
  expect(latestOn(rs, "stable")).toBeNull();
  expect(latestOn(rs, "prerelease")?.version).toBe("1.0.0-rc.6");
});
it("defaults the channel from the running version (R1.3)", () => {
  expect(defaultUpdateSettings("1.0.0-rc.5").channel).toBe("prerelease");
  expect(defaultUpdateSettings("1.0.0").channel).toBe("stable");
});
it("prompts once per version above the skipped one (R4.1)", () => {
  const s = { ...defaultUpdateSettings("1.0.0-rc.5"), skipped: "1.0.0-rc.6" };
  expect(promptFor([r("1.0.0-rc.6")], s, "1.0.0-rc.5")).toBeNull();
  expect(promptFor([r("1.0.0-rc.7"), r("1.0.0-rc.6")], s, "1.0.0-rc.5")?.version).toBe("1.0.0-rc.7");
  expect(promptFor([r("1.0.0-rc.7")], { ...s, autoUpdate: true }, "1.0.0-rc.5")).toBeNull();
});
it("auto-update ignores skipped but never installs failed (R5.2)", () => {
  const s = { ...defaultUpdateSettings("1.0.0-rc.5"), autoUpdate: true, skipped: "1.0.0-rc.6", failed: "1.0.0-rc.7" };
  expect(autoUpdateTo([r("1.0.0-rc.6")], s, "1.0.0-rc.5")?.version).toBe("1.0.0-rc.6");
  expect(autoUpdateTo([r("1.0.0-rc.7"), r("1.0.0-rc.6")], s, "1.0.0-rc.5")).toBeNull();
});
```

- [ ] **Step 2: Run** `npx vitest run src/shared/version.test.ts src/shared/updates.test.ts`. Expected: FAIL (modules missing).
- [ ] **Step 3: Implement both modules.** `parseVersion` uses `/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.(0|[1-9]\d*))?$/`. `parseReleases` checks every R1.4 field: `tag_name` is `v` plus a valid version, not `draft`, version ≥ `FIRST_UPDATER`, exactly one asset named `zipName(v)` with `size > 0`, `digest` matching `/^sha256:[0-9a-f]{64}$/`, and `browser_download_url` starting with `downloadPrefix(v)`. `sha256` keeps the hex only. `autoUpdateTo` returns `latestOn` unless that version equals `failed` (it then returns null; it never falls back to an older one). No Node imports in either file.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): parse versions and github releases`

### Task 2: Update settings

**Files:**
- Modify: `src/main/settings.ts`
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Consumes: `UpdateSettings`, `defaultUpdateSettings`, `parseVersion` (Task 1)
- Produces: `loadUpdateSettings(dataDir: string, running: string): UpdateSettings` (never throws) and `saveUpdateSettings(dataDir: string, s: UpdateSettings): void` (through `update()`, keeping other keys)

- [ ] **Step 1: Write the failing tests** in `settings.test.ts`:
```ts
it("reads missing or malformed update settings as defaults (R12)", () => {
  write({ updates: { channel: "beta", checkAtStartup: "yes", autoUpdate: true, skipped: "v1", failed: 3, lastChecked: 5 } });
  expect(loadUpdateSettings(dir, "1.0.0-rc.5")).toEqual({
    channel: "prerelease", checkAtStartup: true, autoUpdate: true, skipped: null, failed: null, lastChecked: null,
  });
});
it("saves update settings and keeps the other keys", () => {
  write({ lastProject: "D:\\p" });
  saveUpdateSettings(dir, { ...defaultUpdateSettings("1.0.0"), skipped: "1.0.1" });
  expect(readJson().lastProject).toBe("D:\\p");
  expect(loadUpdateSettings(dir, "1.0.0").skipped).toBe("1.0.1");
});
```
- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** the two functions in `settings.ts`, field by field. `skipped` and `failed` must pass `parseVersion`. `lastChecked` must be a string that `Date.parse` accepts.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): save update settings`

### Task 3: Checking for releases

**Files:**
- Create: `src/main/updates/check.ts`
- Test: `src/main/updates/check.test.ts`

**Interfaces:**
- Consumes: `parseReleases`, `RELEASES_API` (Task 1)
- Produces:
  - `updateFeed(env: NodeJS.ProcessEnv, packaged: boolean): string`. R13.2: `GRACE_UPDATE_FEED` only when not packaged, and only an absolute file path or a URL starting with `http://127.0.0.1:`. Anything else falls back to `RELEASES_API`.
  - `fetchReleases(feed: string, timeoutMs?: number): Promise<Release[]>`. Rejects with an `Error` whose message is the R2.5 `<reason>`.

- [ ] **Step 1: Write the failing tests.** Serve answers from a local `http.createServer` on `127.0.0.1:0`:
```ts
it("reads releases from a URL and from a file", async () => { /* both give the same parsed list */ });
it("fails with the status on HTTP errors", async () => {
  await expect(fetchReleases(url403)).rejects.toThrow("GitHub answered 403");
});
it("fails on an unreadable answer", async () => {
  await expect(fetchReleases(urlHtml)).rejects.toThrow("GitHub's answer could not be read");
});
it("times out", async () => {
  await expect(fetchReleases(urlHang, 200)).rejects.toThrow("GitHub did not answer in time");
});
it("fails offline", async () => {
  await expect(fetchReleases("http://127.0.0.1:9/")).rejects.toThrow("GitHub could not be reached");
});
it("honours GRACE_UPDATE_FEED only when not packaged", () => {
  expect(updateFeed({ GRACE_UPDATE_FEED: "http://127.0.0.1:5000/r" }, false)).toBe("http://127.0.0.1:5000/r");
  expect(updateFeed({ GRACE_UPDATE_FEED: "http://127.0.0.1:5000/r" }, true)).toBe(RELEASES_API);
  expect(updateFeed({ GRACE_UPDATE_FEED: "https://evil.example/r" }, false)).toBe(RELEASES_API);
});
```
- [ ] **Step 2: Run** `npx vitest run src/main/updates/check.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.** Use `fetch` with `AbortSignal.timeout(timeoutMs ?? 15000)` and headers `Accept: application/vnd.github+json` and `User-Agent: Grace`. A file feed is read with `readFile`.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): check github releases`

### Task 4: Preflight and the list of Grace's files

**Files:**
- Modify: `forge.config.mts` (a `hooks.postPackage`)
- Create: `src/main/updates/preflight.ts`
- Test: `src/main/updates/preflight.test.ts`

**Interfaces:**
- Produces:
  - `APP_FILES = "app-files.json"`
  - `preflight(installDir: string, zipSize: number, freeBytes?: (dir: string) => Promise<number>): Promise<string | null>`. Returns null when every R8.1 check passes, otherwise the message to show. `freeBytes` defaults to `fs.statfs` (`bavail * bsize`).
  - Every packaged build has `resources/app-files.json`: a JSON array of the packaged folder's top-level names.

- [ ] **Step 1: Write the failing tests** against temp folders that hold a fake `resources/app-files.json`:
```ts
it("passes a folder that holds only Grace's files", async () => {
  expect(await preflight(install, 100, async () => 1e9)).toBeNull();
});
it("ignores desktop.ini and Thumbs.db", async () => { /* add both, still null */ });
it("refuses a folder shared with other files (R8.1)", async () => {
  writeFileSync(path.join(install, "taxes.xlsx"), "");
  expect(await preflight(install, 100, async () => 1e9)).toBe(
    "Grace's folder also holds other files (taxes.xlsx). Put Grace's files in a folder of their own to update it.",
  );
});
it("names at most 3 other files", async () => { /* 5 extra files: message lists 3 */ });
it("refuses a parent it cannot write to", async () => { /* parent made read-only via icacls; message starts "Grace can't update itself in " */ });
it("asks for 4 times the zip size", async () => {
  expect(await preflight(install, 100, async () => 399)).toMatch(/^Not enough free space/);
  expect(await preflight(install, 100, async () => 400)).toBeNull();
});
it("refuses without a file list", async () => { /* no app-files.json: same message as the shared folder, listing the unknown names */ });
```
- [ ] **Step 2: Run** `npx vitest run src/main/updates/preflight.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement `preflight`.** The unwritable message is R8.1's, verbatim. The free-space message is `Not enough free space on <drive>: Grace needs <N> MB to update.`, with N computed from the zip size (my wording; the spec leaves it open). The writable check makes and removes `<parent>/.grace-write-test-<pid>`.
- [ ] **Step 4: Add the Forge hook.** `postPackage(_config, { outputPaths })` writes, for each output path, `resources/app-files.json` = `JSON.stringify(readdirSync(out))`.
- [ ] **Step 5: Run** the unit tests: PASS. Then run `npx electron-forge package` and check that `out/Grace-win32-x64/resources/app-files.json` lists `Grace.exe`, `resources` and `locales`.
- [ ] **Step 6: Commit** `feat(updates): check the install folder before an update`

### Task 5: Download, check and unpack

**Files:**
- Create: `src/main/updates/fetchZip.ts`
- Test: `src/main/updates/fetchZip.test.ts`

**Interfaces:**
- Produces:
  - `download(url: string, dest: string, onProgress: (received: number) => void, signal: AbortSignal): Promise<void>`. Follows redirects, streams to disk, rejects on a non-2xx answer or an abort, and leaves no partial file behind.
  - `verify(file: string, size: number, sha256: string): Promise<boolean>`
  - `TAR = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe")`
  - `unpack(zip: string, dir: string): Promise<void>`. Runs `TAR -xf <zip> -C <dir>` without a shell. Rejects unless `dir/Grace.exe` and `dir/resources/app.asar` exist.

- [ ] **Step 1: Write the failing tests.** Make test zips with `TAR -a -cf x.zip -C <src> .`. Use a source folder holding `Grace.exe` and `resources/app.asar` (empty files) under a path like `…\Zoë dir\`:
```ts
it("downloads with progress and follows a redirect", async () => { /* local server 302 → file; last onProgress === size */ });
it("deletes the partial file on abort", async () => { /* abort mid-stream: rejects, dest absent */ });
it("verifies size and sha256", async () => {
  expect(await verify(zip, size, sha)).toBe(true);
  expect(await verify(zip, size, "0".repeat(64))).toBe(false);
  expect(await verify(zip, size + 1, sha)).toBe(false);
});
it("unpacks into a path with spaces and non-ASCII letters", async () => { /* resolves; files present */ });
it("rejects a zip without Grace.exe", async () => { await expect(unpack(otherZip, dir)).rejects.toThrow(); });
it("never writes outside the target folder", async () => {
  // zip made with TAR -a -cf evil.zip -s ",^,../," f.txt
  await unpack(evilZip, dir).catch(() => {});
  expect(existsSync(path.join(dir, "..", "f.txt"))).toBe(false);
});
```
- [ ] **Step 2: Run** `npx vitest run src/main/updates/fetchZip.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.** `fetch` plus `stream.pipeline` to `createWriteStream`, hashing with `createHash("sha256")` over a read stream, `execFile(TAR, …)`.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): download, check and unpack a release`

### Task 6: Switch script, part 1: back up, restore, swap, result

**Files:**
- Create: `src/main/updates/switch.ps1`, `src/main/updates/switch.ts`
- Test: `src/main/updates/switch.test.ts` (Windows only: `describe.runIf(process.platform === "win32")`)

**Interfaces:**
- Consumes: `UpdateSettings` (Task 1)
- Produces:
  - `interface SwitchPlan { pid: number; from: string; to: string; rollback: boolean; restore: boolean; installDir: string; stagingDir: string; dataDir: string; updates: UpdateSettings; exitTimeoutSec: number; startTimeoutSec: number; keepBackups: number }`
  - `POWERSHELL = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "WindowsPowerShell", "v1.0", "powershell.exe")`
  - `startSwitch(plan: SwitchPlan): void`. Writes `plan.json` and `switch.ps1` (the script is bundled with `import script from "./switch.ps1?raw"`) into `plan.stagingDir`. Spawns `POWERSHELL -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File <staging>\switch.ps1`, `detached`, `windowsHide`, `stdio: "ignore"`, then calls `unref()`.
  - The script's output, a contract (R11.2):
    - `<dataDir>\update-result.json`: `{ from, to, rollback, ok, reverted, error, updates }`
    - `<dataDir>\backups\<version>\backup.json`: `{ version, savedAt }`
    - `<stagingDir>\switching`: the script's own process id while it runs

- [ ] **Step 1: Write the failing tests.** Each test builds:
  - `<tmp>\Grace dir Zoë\` (install, with `Grace.exe` and a `v.txt` holding the version)
  - `<tmp>\Grace dir Zoë.update\app\` (the new app, likewise)
  - a data folder with `settings.json`, `projects\p\runs\r\run.json`, `Cache\x` and `Local Storage\y`

  The stand-in `Grace.exe` is a copy of `process.execPath`. The test passes `NODE_OPTIONS=--require <stub.js>` in the environment it spawns PowerShell with. When it starts, the stub writes `<its folder>.update\started` (unless `noStart.flag` sits next to it) and then stays alive until the test kills it. The test starts the old stand-in itself and puts its pid in the plan. It uses `startTimeoutSec: 20`, then kills the stand-in so the script can finish. It waits for `update-result.json` with a 60 s test timeout.
```ts
it("swaps the install folder and starts the new version", async () => {
  expect(read(install, "v.txt")).toBe("1.0.0-rc.6");
  expect(result()).toMatchObject({ from: "1.0.0-rc.5", to: "1.0.0-rc.6", ok: true, reverted: false });
  expect(existsSync(`${install}.old`)).toBe(true);
});
it("backs up the data before the swap, without caches (R9.1)", async () => {
  const b = path.join(data, "backups", "1.0.0-rc.5");
  expect(existsSync(path.join(b, "projects", "p", "runs", "r", "run.json"))).toBe(true);
  expect(existsSync(path.join(b, "Local Storage", "y"))).toBe(true);
  expect(existsSync(path.join(b, "Cache"))).toBe(false);
  expect(JSON.parse(read(b, "backup.json")).version).toBe("1.0.0-rc.5");
});
it("restores the target's saved data when the plan says so (R9.3)", async () => { /* backups\1.0.0-rc.4\settings.json content is now data\settings.json; backups\ itself untouched */ });
it("keeps the copies of the 3 most recent versions (R9.5)", async () => { /* rc.2, rc.3, rc.4 present (savedAt ordered) → after rc.5→rc.6: rc.3, rc.4, rc.5 */ });
it("puts the old folder and data back when a rename fails (R10.4)", async () => {
  // another process holds a file in the install folder open without sharing
  expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
  expect(result()).toMatchObject({ ok: false, reverted: false });
});
it("gives up when Grace does not exit (R10.2)", async () => { /* exitTimeoutSec: 2, old stand-in kept alive: result ok false, install untouched, no backup */ });
it("writes update-result.json without a byte order mark", async () => {
  expect(readFileSync(path.join(data, "update-result.json"))[0]).not.toBe(0xef);
});
```
- [ ] **Step 2: Run** `npx vitest run src/main/updates/switch.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement `switch.ps1`** in the order of R10.2–R10.5. Read the plan with `Get-Content -Raw -Encoding UTF8 | ConvertFrom-Json`. Write `switching` first and delete it last. Wait for "no process runs from the install folder" with `Get-Process | Where-Object { $_.Path -like "$install\*" }` in a loop. Copy with `robocopy /E`, leaving out the R9.1 folders and files (exit codes below 8 mean success). Write every JSON file with `[IO.File]::WriteAllText($p, $json, (New-Object Text.UTF8Encoding $false))`. Wrap each step in `try`/`catch`: any failure from the backup on undoes what was done (R9.6, R10.4), writes `ok: false` with the error, and starts the old `Grace.exe`. This part stops after "start the new version", with no start check yet. Then implement `startSwitch` in `switch.ts`.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): switch versions with a powershell script`

### Task 7: Switch script, part 2: the new version must start

**Files:**
- Modify: `src/main/updates/switch.ps1`
- Test: `src/main/updates/switch.test.ts`

**Interfaces:**
- Consumes: Task 6's plan and stand-ins
- Produces: R10.6. `<stagingDir>\started` is the signal. A version that does not start ends with `reverted: true` and the error `<to> didn't start within 2 minutes`; the wording is fixed at 2 minutes, since `startTimeoutSec` exists only for tests.

- [ ] **Step 1: Write the failing tests:**
```ts
it("switches back when the new version does not start in time (R10.6)", async () => {
  // noStart.flag in the new app folder, startTimeoutSec: 3
  expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
  expect(existsSync(`${install}.failed`)).toBe(true);
  expect(read(data, "settings.json")).toBe(originalSettings); // data put back from backups\1.0.0-rc.5
  expect(result()).toMatchObject({ ok: false, reverted: true, error: "1.0.0-rc.6 didn't start within 2 minutes" });
});
it("deletes old backups only after the new version started", async () => { /* in the revert case, rc.2 survives */ });
```
- [ ] **Step 2: Run** the file. Expected: the new tests FAIL.
- [ ] **Step 3: Implement** the wait for `started` and the revert. Stop the processes with `taskkill /T /F` per process found under the install folder. Move retention (R9.5) after a confirmed start.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): switch back when a new version does not start`

### Task 8: Startup side of a switch

**Files:**
- Create: `src/main/updates/startup.ts`
- Test: `src/main/updates/startup.test.ts`

**Interfaces:**
- Consumes: `UpdateSettings`, `Backup`, `parseVersion` (Task 1), the script's files (Tasks 6–7)
- Produces:
  - `interface SwitchResult { from: string; to: string; rollback: boolean; ok: boolean; reverted: boolean; error: string | null; updates: UpdateSettings | null }`
  - `switchInProgress(installDir: string): boolean`: `<install>.update\switching` holds the pid of a live process (`process.kill(pid, 0)`)
  - `markStarted(installDir: string, running: string): void`: writes `<install>.update\started` when `<install>.update\plan.json` has `to === running`
  - `takeResult(dataDir: string): SwitchResult | null`: reads and deletes the file; tolerates a BOM; malformed → null (and deleted)
  - `resultNotice(r: SwitchResult): string`
  - `settingsAfter(r: SwitchResult, current: UpdateSettings): UpdateSettings`
  - `removeLeftovers(installDir: string): Promise<void>`: once no switch is in progress, removes `<install>.old`, `<install>.failed` and `<install>.update`
  - `listBackups(dataDir: string): Backup[]`: newest `savedAt` first; skips folders whose `backup.json` is missing or invalid

- [ ] **Step 1: Write the failing tests:**
```ts
it("words the notices (R10.7)", () => {
  expect(resultNotice(ok("1.0.0-rc.5", "1.0.0-rc.6"))).toBe("Updated to 1.0.0-rc.6.");
  expect(resultNotice({ ...ok("1.0.0-rc.6", "1.0.0-rc.5"), rollback: true })).toBe("Rolled back to 1.0.0-rc.5.");
  expect(resultNotice(failed("1.0.0-rc.5", "1.0.0-rc.6", "a file is in use")))
    .toBe("Couldn't switch to 1.0.0-rc.6: a file is in use. You are still on 1.0.0-rc.5.");
});
it("holds a rollback: carried settings, auto-update off, from skipped (R10.7)", () => {
  const s = settingsAfter({ ...ok("1.0.0-rc.6", "1.0.0-rc.5"), rollback: true, updates: carried }, restored);
  expect(s).toEqual({ ...carried, autoUpdate: false, skipped: "1.0.0-rc.6" });
});
it("remembers a version that did not start (R10.6)", () => {
  expect(settingsAfter({ ...failed("1.0.0-rc.5", "1.0.0-rc.6", "x"), reverted: true }, cur).failed).toBe("1.0.0-rc.6");
});
it("reads a result with a BOM and deletes it", () => { /* ... */ });
it("sees a live switch and ignores a dead one (R10.9)", () => { /* switching = own pid → true; = 999999 → false */ });
it("marks the start only for the version the plan installs", () => { /* plan.to rc.6, running rc.6 → started; running rc.5 → none */ });
it("removes a partial download left by a killed Grace (R10.8)", async () => {
  // <install>.update\Grace-…zip partial, no switching file
  await removeLeftovers(install);
  expect(existsSync(`${install}.update`)).toBe(false);
});
it("lists backups newest first", () => { /* ... */ });
```
- [ ] **Step 2: Run** `npx vitest run src/main/updates/startup.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.** `removeLeftovers` polls `switchInProgress` every second, up to 3 minutes, then removes with `rm({ recursive: true, force: true })` and swallows errors.
- [ ] **Step 4: Run** it again. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): read the result of a switch at startup`

### Task 9: The Updater

**Files:**
- Create: `src/main/updates/updater.ts`
- Modify: `src/main/runner.ts` (add `isBusy(): boolean`, returning the private `busy`), `src/shared/events.ts`
- Test: `src/main/updates/updater.test.ts`

**Interfaces:**
- Consumes: Tasks 1–8
- Produces:
  - `AppEvent` gains:
    - `{ type: "updates"; view: UpdateView }`
    - `{ type: "updatePrompt"; release: Release }`
    - `{ type: "updateProgress"; progress: UpdateProgress | null }`
  - `interface UpdaterDeps { dataDir: string; installDir: string; running: string; packaged: boolean; feed: string; isBusy(): boolean; emit(e: AppEvent): void; quit(): void; openExternal(url: string): Promise<void>; fetchReleases?: typeof fetchReleases; preflight?: typeof preflight; download?: typeof download; verify?: typeof verify; unpack?: typeof unpack; startSwitch?: typeof startSwitch }`
  - `class Updater`:
    - `view(): UpdateView`
    - `startup(result: SwitchResult | null): Promise<void>`
    - `check(): Promise<void>`
    - `setSettings(patch: { channel?: Channel; checkAtStartup?: boolean; autoUpdate?: boolean }): void`
    - `skip(version: string): void`
    - `install(version: string, useSavedData: boolean): Promise<void>`
    - `cancel(): void`
    - `whatsNew(version: string): Promise<void>`
    - `progress(): UpdateProgress | null`

- [ ] **Step 1: Write the failing tests** with fakes for every injected function and a temp `dataDir`:
```ts
it("startup: applies the result, then checks, then prompts (R2.2, R4.1)", async () => { /* emits notice, updates, updatePrompt rc.6 */ });
it("startup: no check and no request when checkAtStartup is off (R2.3)", async () => { expect(fetchReleases).not.toHaveBeenCalled(); });
it("startup: auto-update installs instead of prompting (R5.2)", async () => { /* install path reached, no updatePrompt */ });
it("startup: no auto-update right after a failed switch (R5.3)", async () => { /* result ok false → no install */ });
it("check: one at a time (R2.4)", async () => { /* two calls, one fetch */ });
it("check: a failure keeps lastChecked and sets the error (R2.5)", async () => {
  expect(u.view().error).toBe("Couldn't check for updates: GitHub answered 403");
});
it("install: refused while busy or not packaged (R6.4)", async () => { /* emits notice "Finish or cancel the run first" / "Updates install only in the packaged app"; nothing downloaded */ });
it("install: a damaged download changes nothing (R8.3)", async () => {
  expect(u.progress()).toEqual({ version: "1.0.0-rc.6", rollback: false, error: "The download was damaged. Try again, or get it from the release page." });
  expect(startSwitch).not.toHaveBeenCalled(); expect(existsSync(staging)).toBe(false);
});
it("install: hands over with the right plan, then quits (R8.5)", async () => {
  expect(startSwitch).toHaveBeenCalledWith(expect.objectContaining({
    from: "1.0.0-rc.5", to: "1.0.0-rc.6", rollback: false, restore: false,
    exitTimeoutSec: 60, startTimeoutSec: 120, keepBackups: 3,
  }));
  expect(quit).toHaveBeenCalled();
});
it("install: a rollback restores a saved copy; an update only when asked (R7.6, R9.3)", async () => { /* ... */ });
it("cancel: removes the staging folder and clears progress (R8.6)", async () => { /* ... */ });
it("whatsNew opens only the release page of a valid version (R4.2)", async () => {
  await u.whatsNew("1.0.0-rc.6"); expect(openExternal).toHaveBeenCalledWith("https://github.com/hpagniez/grace/releases/tag/v1.0.0-rc.6");
  await u.whatsNew("x\" & calc"); expect(openExternal).toHaveBeenCalledTimes(1);
});
```
- [ ] **Step 2: Run** `npx vitest run src/main/updates/updater.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - The staging folder is `${installDir}.update`. A leftover one is removed before the download (R8.2). The zip goes in as `zipName(version)`, unpacked into `app\`.
  - Each step emits `updateProgress`.
  - Download progress is throttled to at most one event every 100 ms.
  - Error messages are the step's message, plus the release page link the renderer adds. The damaged-download wording in the test is mine; the spec only says it must say the download was damaged.
  - `install` accepts only a version in the last successful check's releases. Any other version is refused with a notice.
  - A successful `check` saves `lastChecked` (R2.6). `setSettings` saves, then emits `updates`.
- [ ] **Step 4: Run** it again, and the runner tests: `npx vitest run src/main/updates src/main/runner.runs.test.ts`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(updates): orchestrate checks and installs in main`

### Task 10: IPC, startup wiring and the status bar

**Files:**
- Modify: `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`, `src/renderer/reduce.ts`, `src/renderer/store.ts`, `src/renderer/App.tsx`
- Create: `src/renderer/StatusBar.tsx`, `e2e/updates.spec.ts`, `e2e/feed.ts`
- Test: `src/renderer/StatusBar.test.tsx`, `src/renderer/reduce.test.ts`, `e2e/updates.spec.ts`

**Interfaces:**
- Consumes: `Updater` (Task 9), `switchInProgress`, `markStarted`, `takeResult`, `removeLeftovers` (Task 8), `updateFeed` (Task 3)
- Produces:
  - `CH` gains:
    - `checkUpdates: "updates:check"`
    - `installVersion: "updates:install"`
    - `cancelUpdate: "updates:cancel"`
    - `updateSettings: "updates:settings"`
    - `skipVersion: "updates:skip"`
    - `whatsNew: "updates:whatsNew"`
  - `Api` gains:
    - `checkUpdates(): Promise<void>`
    - `installVersion(version: string, useSavedData: boolean): Promise<void>`
    - `cancelUpdate(): Promise<void>`
    - `setUpdateSettings(patch: { channel?: Channel; checkAtStartup?: boolean; autoUpdate?: boolean }): Promise<void>`
    - `skipVersion(version: string): Promise<void>`
    - `openWhatsNew(version: string): Promise<void>`
  - `AppState` gains `updates: UpdateView | null`, `updatePrompt: Release | null` and `updateProgress: UpdateProgress | null`. The store gains `dismissPrompt(): void`.
  - `StatusBar(props: { state: AppState; api: Api; openSettings(): void })`
  - `e2e/feed.ts`: `feedFile(releases: { version: string; prerelease?: boolean }[]): string` (writes GitHub-shaped JSON with valid digests and URLs, returns the path) and `feedServer(releases): Promise<{ url: string; requests(): number; close(): Promise<void> }>`

- [ ] **Step 1: Write the failing tests.**
  - `StatusBar.test.tsx`:
    - shows `Grace <running>` and `⚙ Settings`
    - shows the marker `1.0.0-rc.6 available` and `Update` only while out of date on the channel, the skipped version included
    - has the accessible name `Grace 1.0.0-rc.5, 1.0.0-rc.6 available`
    - `Update` calls `api.installVersion("1.0.0-rc.6", false)`
  - `reduce.test.ts`: the three new events set their state.
  - `e2e/updates.spec.ts`:
```ts
test("the status bar shows the version on every view (R3.1)", async () => { /* home, then a project: status-bar text contains `Grace ${pkg.version}` */ });
test("out of date on the pre-release channel shows the marker (R3.2)", async () => {
  const { win } = await launch(undefined, { GRACE_UPDATE_FEED: feedFile([{ version: "1.0.0-rc.99", prerelease: true }]) });
  await expect(win.getByTestId("update-marker")).toHaveText(/1\.0\.0-rc\.99 available/);
});
```
- [ ] **Step 2: Run** `npx vitest run src/renderer/StatusBar.test.tsx src/renderer/reduce.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement the IPC and the main wiring.** In `main.ts`:
  - the installDir is `path.dirname(process.execPath)`
  - when packaged and `switchInProgress(installDir)`, show `dialog.showMessageBoxSync({ message: "Grace is finishing an update and starts on its own in a moment." })` and quit before `createWindow`
  - in `reveal()`, call `markStarted` once
  - after `createWindow`: `takeResult`, then `void updater.startup(result)` (never awaited before the project reopens), then `void removeLeftovers(installDir)` when packaged
  - `CH.ready` also emits `updates` and `updateProgress`
  - the IPC handlers validate their arguments (Global Constraints)
- [ ] **Step 4: Implement the renderer.** Add `StatusBar` at the bottom of `App`, outside the `home()` switch, so it shows on every view. Each item is a `<button>` with a `data-testid` (`update-marker`, `update-link`, `status-version`, `settings-button`).
- [ ] **Step 5: Run** the unit tests: PASS. Then `npm run e2e -- e2e/updates.spec.ts`: PASS.
- [ ] **Step 6: Commit** `feat(updates): show the version and update marker in a status bar`

### Task 11: Startup modal and progress dialog

**Files:**
- Create: `src/renderer/UpdateModal.tsx`, `src/renderer/UpdateProgress.tsx`
- Modify: `src/renderer/App.tsx`
- Test: `src/renderer/UpdateModal.test.tsx`, `src/renderer/UpdateProgress.test.tsx`, `e2e/updates.spec.ts`

**Interfaces:**
- Consumes: `state.updatePrompt`, `state.updateProgress`, `dismissPrompt`, `Api` (Task 10)
- Produces: `UpdateModal(props: { state: AppState; api: Api; dismiss(): void })` and `UpdateProgress(props: { state: AppState; api: Api })`

- [ ] **Step 1: Write the failing tests.**
  - `UpdateModal.test.tsx`, with R4.2 copy verbatim:
```ts
expect(screen.getByRole("heading")).toHaveTextContent("Grace 1.0.0-rc.6 is available");
expect(dialog).toHaveTextContent("You have 1.0.0-rc.5. Download: 219 MB. Your data is backed up before Grace restarts.");
// size 218_557_130 → 219 MB (Math.round(bytes / 1e6))
expect(screen.getByRole("button", { name: "Update and restart" })).toHaveFocus();
```
  - Clicks and keys:
    - `Skip this version` → `api.skipVersion("1.0.0-rc.6")` and `dismiss()`
    - with the checkbox ticked, `Update and restart` → `api.setUpdateSettings({ autoUpdate: true })` then `api.installVersion("1.0.0-rc.6", false)`
    - with the checkbox ticked, `Skip this version` → no `setUpdateSettings`
    - Esc → `dismiss()` and nothing saved
    - `What's new` → `api.openWhatsNew("1.0.0-rc.6")`
  - `UpdateProgress.test.tsx`:
    - shows `Updating to 1.0.0-rc.6` or `Rolling back to 1.0.0-rc.4`
    - shows `Downloading… 92 of 219 MB`, then `Checking…` and `Unpacking…`
    - `Cancel` → `api.cancelUpdate()`
    - an error shows the message, a `release page` link (→ `openWhatsNew`) and `Close`
  - e2e, using `feedFile` with `1.0.0-rc.99` (copy from the spec's Gherkin):
```ts
test("skipping a version keeps the marker but not the modal (R4.3)", async () => { /* skip → relaunch same userData → no modal, marker shown */ });
test("a newer version after a skip shows the modal again", async () => { /* skipped rc.98, feed rc.99 → modal */ });
test("closing the modal without a choice asks again (R4.5)", async () => { /* Esc → relaunch → modal */ });
test("a failed check is quiet (R2.5)", async () => { /* feed path to a missing file → no modal, no dialog */ });
```
- [ ] **Step 2: Run** `npx vitest run src/renderer/UpdateModal.test.tsx src/renderer/UpdateProgress.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** both dialogs on the `PropertiesDialog` pattern. Focus returns where it was on close (R4.6). The progress dialog has no close by Esc while a step runs.
- [ ] **Step 4: Run** the unit tests: PASS. Then `npm run e2e -- e2e/updates.spec.ts`: PASS.
- [ ] **Step 5: Commit** `feat(updates): ask at startup and show install progress`

### Task 12: Settings page and switch dialogs

**Files:**
- Create: `src/renderer/Settings.tsx`, `src/renderer/SwitchDialog.tsx`, `src/renderer/lastChecked.ts`
- Modify: `src/renderer/App.tsx`
- Test: `src/renderer/Settings.test.tsx`, `src/renderer/SwitchDialog.test.tsx`, `src/renderer/lastChecked.test.ts`, `e2e/updates.spec.ts`

**Interfaces:**
- Consumes: `UpdateView`, `Backup`, `compareVersions`, `Api` (Tasks 1, 10)
- Produces:
  - `Settings(props: { state: AppState; api: Api; back(): void })`
  - `SwitchDialog(props: { target: Release; running: string; backup: Backup | null; onConfirm(useSavedData: boolean): void; onCancel(): void })`
  - `lastCheckedText(iso: string | null, now: number): string` (through `Intl.RelativeTimeFormat("en", { numeric: "auto" })`)

- [ ] **Step 1: Write the failing tests.**
  - `lastChecked.test.ts`: `lastCheckedText(iso2minAgo, now)` → `"Last checked 2 minutes ago"`.
  - `SwitchDialog.test.tsx`, with R7 copy verbatim:
    - **Rollback with a copy:** title `Roll back to 1.0.0-rc.4?`. Text `Grace backs up your current data, then restores the data saved when you last used 1.0.0-rc.4 (2026-10-29).`. Amber box text `Going back to an older version may break your data. Runs, settings and secrets added since then are not in the restored copy; they stay in the backup of 1.0.0-rc.5, and come back if you install 1.0.0-rc.5 again and use its saved data.`. `Cancel` focused.
    - **Rollback without a copy:** text `Grace backs up your current data. 1.0.0-rc.4 has no saved data on this computer, so it starts from your current data, which it may not read correctly.`, and the amber box holds only its first sentence.
    - **Newer version with a copy (R7.6):** title `Install 1.0.0-rc.6?`. Choices `Use the data you had in 1.0.0-rc.6 (2026-10-29)` and `Keep your current data` (checked). `Install and restart` → `onConfirm(false)`; choosing the first → `onConfirm(true)`.
  - `Settings.test.tsx`:
    - The table lists the installable releases on the channel plus the running row, newest first.
    - The running row is tagged `installed` and has no button. A row with a backup is tagged `data saved`.
    - The buttons are `Install` above the running version and `Roll back` below it.
    - Without a check, it says `Check for updates to see other versions`.
    - Changing the channel → `api.setUpdateSettings({ channel: "stable" })`.
    - `Update automatically, without asking` is disabled while `Check for updates` is unticked.
    - Every switch button is disabled with `Finish or cancel the run first` while `state.busy` or `state.run.active`, and with `Updates install only in the packaged app` when `!packaged`.
    - `Install` on a newer version with no backup → `api.installVersion(v, false)` directly.
    - `Roll back` → the `SwitchDialog`.
    - A check error shows `Couldn't check for updates: <reason>`.
  - e2e (spec Gherkin):
```ts
test("default channel follows the running version", async () => { /* Stable and pre-releases checked */ });
test("version list and dev-build install button (R6.3, R6.4)", async () => { /* rc.99 row Install disabled, title "Updates install only in the packaged app" */ });
test("Back returns to the open project unchanged (R6.1)", async () => { /* open project, settings, Back → tree still shown */ });
test("no network when the startup check is off (R2.3)", async () => {
  const feed = await feedServer([{ version: "1.0.0-rc.99", prerelease: true }]);
  // launchWithSettings({ updates: { checkAtStartup: false } }) with GRACE_UPDATE_FEED = feed.url
  // wait 3 s → feed.requests() === 0; click "Check for updates" → 1
});
```
- [ ] **Step 2: Run** `npx vitest run src/renderer/Settings.test.tsx src/renderer/SwitchDialog.test.tsx src/renderer/lastChecked.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.** In `App`, a `settingsOpen` signal replaces the toolbar, tree and panel, or the home screen, with `Settings`. The status bar stays. `back()` and Esc on the page close it. The project's state lives in the store, so it comes back unchanged. The layout follows mockup part 4. For the no-network test, give `launchWithSettings` in `e2e/helpers.ts` an optional second parameter, `extraEnv`, passed on to `launch`.
- [ ] **Step 4: Run** the unit tests: PASS. Then `npm run e2e -- e2e/updates.spec.ts`: PASS.
- [ ] **Step 5: Commit** `feat(updates): settings page to change channel and switch versions`

### Task 13: Whole-branch checks

**Files:** none new, unless a check fails.

- [ ] **Step 1:** `npm run lint` and `npm run typecheck`: no errors.
- [ ] **Step 2:** `npm test`: all pass. Then, never at the same time, `npm run e2e`: all pass. Paste both summaries in the PR.
- [ ] **Step 3:** Go through the spec's acceptance criteria one by one and name the test that covers each. The scenarios marked "Packaged app, checked by hand" are covered by Tasks 6–7's script tests; their by-hand run is part of the release check of the first release after `1.0.0-rc.4` (installing it from `1.0.0-rc.4`), recorded in `docs/checks/`.
- [ ] **Step 4:** Open the PR into `develop` with `Closes #77`, and move the #77 card to `building` (AGENTS.md).
