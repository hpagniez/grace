# Switches Grace to another version (docs/specs/2026-10-28-app-updates.md R9, R10).
# Grace writes this script and plan.json into the staging folder <install>.update, starts it and quits.
# Windows PowerShell 5.1 reads a script without a byte order mark in the ANSI code page: keep it ASCII.
param([switch]$Detached)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

function Write-Utf8($path, $text) {
  # every file this script writes is UTF-8 without a byte order mark
  [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false))
}

# Starts a program directly, not through the shell (Start-Process reads a [ in a path as a
# wildcard), in this process's environment. A console program gets no window; Grace opens its own.
function Start-Program($exe, $arguments, $folder) {
  $psi = New-Object Diagnostics.ProcessStartInfo $exe, $arguments
  $psi.WorkingDirectory = $folder
  $psi.UseShellExecute = $false
  $psi.CreateNoWindow = $true
  [Diagnostics.Process]::Start($psi)
}

$switching = Join-Path $PSScriptRoot 'switching'

if (-not $Detached) {
  # Grace started this copy, which dies when Grace exits (Node kills the processes it started).
  # A copy started from here is outside that group and outlives Grace. That copy writes its id to
  # switching first: Grace quits only once the file exists, the proof that the switch runs (R10.9).
  $arguments = "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Detached"
  $p = Start-Program (Join-Path $PSHOME 'powershell.exe') $arguments $PSScriptRoot
  for ($i = 0; $i -lt 100; $i++) {
    if (Test-Path -LiteralPath $switching) { exit 0 }
    if ($p.HasExited) { exit 1 }
    Start-Sleep -Milliseconds 100
  }
  # Grace stays: a copy that ran late would switch once the person quits Grace, unasked
  try { $p.Kill() } catch {}
  exit 1
}

function Write-Json($path, $value) { Write-Utf8 $path (ConvertTo-Json $value -Depth 5) }

# The innermost error text, without its final period: Grace puts it in a sentence (R10.7).
function Get-Message($record) {
  $e = $record.Exception
  while ($e.InnerException) { $e = $e.InnerException }
  $e.Message.Trim().TrimEnd('.')
}

function Remove-Tree($path) {
  if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
}

# A saved copy without its backup.json is never restored or kept (R9.3, R9.5): removing that file
# first means a copy removed half-way is never trusted.
function Remove-Copy($dir) {
  $meta = Join-Path $dir 'backup.json'
  if (Test-Path -LiteralPath $meta) { Remove-Item -LiteralPath $meta -Force }
  Remove-Tree $dir
}

# Undoing must not leave Grace without an install folder: retry while an antivirus or a dying
# process holds a file, as long as the wait for Grace to quit (60 s).
function Move-Back($from, $to) {
  $deadline = [DateTime]::UtcNow.AddSeconds($plan.exitTimeoutSec)
  for (;;) {
    try { [IO.Directory]::Move($from, $to); return } catch { if ([DateTime]::UtcNow -gt $deadline) { throw } }
    Start-Sleep 1
  }
}

# Copies the folder $from into $to, leaving out the full paths in $skip.
function Copy-Tree($from, $to, $skip) {
  # Full paths leave out those entries only, not same-named ones deeper down (a project's Cache).
  # /XJ: never follow a junction out of the folder. /R:2 /W:1: robocopy retries a million times by default.
  $a = @($from, $to, '/E', '/XJ', '/R:2', '/W:1', '/NP', '/NFL', '/NDL', '/NJH', '/NJS') + '/XD' + $skip + '/XF' + $skip
  & $robocopy $a | Out-Null
  # exit codes below 8 mean success
  if ($LASTEXITCODE -ge 8) { throw "copying the data failed (robocopy exit code $LASTEXITCODE)" }
}

# R9.3: the data folder's contents, except backups\, become the saved copy $copy.
function Set-Data($copy) {
  $failed = $null
  foreach ($e in Get-ChildItem -LiteralPath $data -Force) {
    if ($e.Name -ne 'backups') {
      try { Remove-Item -LiteralPath $e.FullName -Recurse -Force } catch { if (-not $failed) { $failed = $_ } }
    }
  }
  # copy even when a removal failed: undoing a restore must put every file of the copy back
  Copy-Tree $copy $data @(Join-Path $copy 'backup.json')
  if ($failed) { throw $failed }
}

function Write-Result($ok, $err, $reverted) {
  Write-Json (Join-Path $data 'update-result.json') ([ordered]@{
      from = $plan.from; to = $plan.to; rollback = $plan.rollback; ok = $ok
      reverted = $reverted; error = $err; updates = $plan.updates
    })
}

# Grace, its helpers (renderer, GPU, crash handler) and the bundled Java all run from the install
# folder. StartsWith, not -like: a [ in a folder name is a wildcard. Path asks the process on each
# read, so it is read once: a process that exits in between has none the second time.
function Get-Grace {
  Get-Process | Where-Object { $p = $_.Path; $p -and $p.StartsWith("$install\", [StringComparison]::OrdinalIgnoreCase) }
}

function Start-Grace { Start-Program (Join-Path $install 'Grace.exe') '' $install }

# A switch removes a leftover .old or .failed: only one Grace made, which holds the build's file list.
function Assert-Leftover($dir) {
  if ((Test-Path -LiteralPath $dir) -and -not (Test-Path -LiteralPath (Join-Path $dir 'resources\app-files.json'))) {
    throw "$dir is in the way and Grace didn't make it; move or rename it, then try again"
  }
}

# Moved into the staging folder whole, then removed: a removal cut short by a held file never leaves
# half a Grace folder without its file list next to the install, which Grace would then refuse (R10.8).
function Remove-Leftover($dir, $name) {
  if (-not (Test-Path -LiteralPath $dir)) { return }
  $inside = Join-Path $PSScriptRoot $name
  Remove-Tree $inside
  Move-Back $dir $inside  # retried: an antivirus may hold a file for a moment
  # Once inside the staging folder it is out of the way; Grace removes what is left later (R10.8).
  try { Remove-Tree $inside } catch {}
}

# Each flag says what to undo (R9.6, R10.4, R10.6). Set before the try: the catch reads them all.
$plan = $null; $exited = $false; $copying = $false; $copied = $false; $restored = $false
$movedOld = $false; $movedNew = $false; $startedNew = $false; $ok = $false
try {
  try {
    # first: the proof that the switch runs (R10.9)
    Write-Utf8 $switching "$PID"
    $plan = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'plan.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    # no trailing backslash: PowerShell would quote it into an escaped quote for robocopy
    $install = $plan.installDir.TrimEnd('\')
    $data = $plan.dataDir.TrimEnd('\')
    $app = Join-Path $PSScriptRoot 'app'
    $old = "$install.old"
    $aside = "$install.failed"
    $dest = $aside  # where a new version that did not start goes; the staging folder if .failed is stuck
    $backups = Join-Path $data 'backups'
    $backup = Join-Path $backups $plan.from
    $robocopy = Join-Path $env:SystemRoot 'System32\Robocopy.exe'
    $taskkill = Join-Path $env:SystemRoot 'System32\taskkill.exe'

    # R10.2: wait for Grace to exit. On timeout: a failure result, nothing else, Grace still runs.
    $deadline = [DateTime]::UtcNow.AddSeconds($plan.exitTimeoutSec)
    while (Get-Grace) {
      if ([DateTime]::UtcNow -gt $deadline) { throw "Grace didn't quit within $($plan.exitTimeoutSec) seconds" }
      Start-Sleep -Milliseconds 250
    }
    $exited = $true
    # before touching anything: Grace starts again and nothing else changes
    Assert-Leftover $old
    Assert-Leftover $aside

    # R10.3, in order: back up, restore, rename, rename
    Remove-Leftover $old 'old'  # left by an earlier switch
    $copying = $true
    Remove-Copy $backup  # R9.1: an older copy of this version is replaced
    $skip = 'backups', 'update-result.json', 'Cache', 'Code Cache', 'GPUCache', 'GrShaderCache', 'ShaderCache',
      'GPUPersistentCache', 'DawnGraphiteCache', 'DawnWebGPUCache', 'Crashpad', 'DevToolsActivePort', 'lockfile',
      'SingletonLock'
    Copy-Tree $data $backup @($skip | ForEach-Object { Join-Path $data $_ })
    $savedAt = [DateTime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", [Globalization.CultureInfo]::InvariantCulture)
    Write-Json (Join-Path $backup 'backup.json') ([ordered]@{ version = $plan.from; savedAt = $savedAt })
    $copied = $true

    # R9.3: only a complete copy, one with its backup.json
    $saved = Join-Path $backups $plan.to
    if ($plan.restore -and (Test-Path -LiteralPath (Join-Path $saved 'backup.json'))) {
      $restored = $true  # set first: a restore that fails half-way is undone too
      Set-Data $saved
    }

    # R10.9: a Grace started since the wait shows its message and quits within seconds
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((Get-Grace) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 250 }

    [IO.Directory]::Move($install, $old)
    $movedOld = $true
    [IO.Directory]::Move($app, $install)
    $movedNew = $true

    # R10.5: the result first, since the new version reads it when it starts
    Write-Result $true $null $false
    $grace = Start-Grace
    $startedNew = $true

    # R10.6: the new version writes started once its window shows. One that quit never will.
    $deadline = [DateTime]::UtcNow.AddSeconds($plan.startTimeoutSec)
    for (;;) {
      $quit = $grace.HasExited  # read first: it may write started just before it quits
      if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'started')) { break }
      if ($quit) { throw "$($plan.to) quit right after starting" }
      # the wording stays at 2 minutes: startTimeoutSec is shorter only in tests
      if ([DateTime]::UtcNow -gt $deadline) { throw "$($plan.to) didn't start within 2 minutes" }
      Start-Sleep -Milliseconds 250
    }
    $ok = $true
  } catch {
    $err = Get-Message $_
    # Undo in reverse order, each step on its own so that one failure does not skip the rest. Each
    # failure goes in the result: Grace must not say the old version is back when it may not be.
    if ($startedNew) {
      # R10.6: stop the new version and everything it started, then keep it aside
      try {
        foreach ($p in Get-Grace) { try { & $taskkill /PID $p.Id /T /F | Out-Null } catch {} }
        # A .failed left by an earlier switch that cannot be moved must not keep the new version in
        # place: it goes back into the staging folder instead.
        try { Remove-Leftover $aside 'failed' } catch {
          $err += "; clearing $aside failed: $(Get-Message $_)"
          $dest = $app
        }
        Move-Back $install $dest
      } catch { $err += "; moving $($plan.to) aside failed: $(Get-Message $_)" }
    } elseif ($movedNew) {
      try { Move-Back $install $app } catch { $err += "; moving $($plan.to) aside failed: $(Get-Message $_)" }
    }
    if ($movedOld -and -not (Test-Path -LiteralPath $install)) {
      try { Move-Back $old $install } catch {
        $err += "; putting $($plan.from) back failed: $(Get-Message $_)"
        # never no install folder: the new version goes back in rather than none
        try {
          Move-Back $(if ($startedNew) { $dest } else { $app }) $install
          $err += ", so $($plan.to) stays installed"
        } catch { $err += "; putting $($plan.to) back failed too: $(Get-Message $_)" }
      }
    }
    # R10.4, R10.6: the running version's data back from the copy just made, so a later try does
    # not overwrite the real backup with the target's data
    if ($restored -or $startedNew) {
      try { Set-Data $backup } catch { $err += "; putting the data back failed: $(Get-Message $_)" }
    }
    if ($copying -and -not $copied) { try { Remove-Copy $backup } catch {} }
    if ($plan) { try { Write-Result $false $err $startedNew } catch {} }
    if ($exited) {
      # a Grace other than the new version quits at once while switching names a live process (R10.9)
      Remove-Item -LiteralPath $switching -Force -ErrorAction SilentlyContinue
      try { [void](Start-Grace) } catch {}
    }
  }

  if ($ok) {
    # R9.5, only after a confirmed start: keep the copies of the most recently backed-up versions.
    # A folder without a valid backup.json is not a copy: leave it alone.
    try {
      $copies = foreach ($d in Get-ChildItem -LiteralPath $backups -Directory) {
        try {
          $meta = Get-Content -LiteralPath (Join-Path $d.FullName 'backup.json') -Raw -Encoding UTF8 | ConvertFrom-Json
          New-Object PSObject -Property @{ Path = $d.FullName; SavedAt = [DateTime]$meta.savedAt }
        } catch {}
      }
      $copies | Sort-Object SavedAt -Descending | Select-Object -Skip $plan.keepBackups |
        ForEach-Object { try { Remove-Copy $_.Path } catch {} }
    } catch {}
  }
} finally {
  Remove-Item -LiteralPath $switching -Force -ErrorAction SilentlyContinue
}
