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
  # A copy started from here is outside that group and outlives Grace. Grace waits until switching
  # exists, so it quits only once the real switch runs (R10.9).
  $arguments = "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Detached"
  $p = Start-Program (Join-Path $PSHOME 'powershell.exe') $arguments $PSScriptRoot
  Write-Utf8 $switching "$($p.Id)"
  exit
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

# Undoing must not leave Grace without an install folder: retry while an antivirus holds a file.
function Move-Back($from, $to) {
  for ($i = 1; ; $i++) {
    try { [IO.Directory]::Move($from, $to); return } catch { if ($i -ge 10) { throw }; Start-Sleep 1 }
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

function Write-Result($ok, $err) {
  Write-Json (Join-Path $data 'update-result.json') ([ordered]@{
      from = $plan.from; to = $plan.to; rollback = $plan.rollback; ok = $ok
      reverted = $false; error = $err; updates = $plan.updates
    })
}

# Grace, its helpers (renderer, GPU, crash handler) and the bundled Java all run from the install
# folder. StartsWith, not -like: a [ in a folder name is a wildcard.
function Test-Running {
  foreach ($p in Get-Process) {
    if ($p.Path -and $p.Path.StartsWith("$install\", [StringComparison]::OrdinalIgnoreCase)) { return $true }
  }
  $false
}

function Start-Grace { [void](Start-Program (Join-Path $install 'Grace.exe') '' $install) }

try {
  $plan = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'plan.json') -Raw -Encoding UTF8 | ConvertFrom-Json
  # no trailing backslash: PowerShell would quote it into an escaped quote for robocopy
  $install = $plan.installDir.TrimEnd('\')
  $data = $plan.dataDir.TrimEnd('\')
  $app = Join-Path $PSScriptRoot 'app'
  $old = "$install.old"
  $backups = Join-Path $data 'backups'
  $backup = Join-Path $backups $plan.from
  $robocopy = Join-Path $env:SystemRoot 'System32\Robocopy.exe'

  # R10.2: wait for Grace to exit
  $deadline = [DateTime]::UtcNow.AddSeconds($plan.exitTimeoutSec)
  while (Test-Running) {
    if ([DateTime]::UtcNow -gt $deadline) {
      Write-Result $false "Grace didn't quit within $($plan.exitTimeoutSec) seconds"
      return
    }
    Start-Sleep -Milliseconds 250
  }

  # R10.3, in order: back up, restore, rename, rename. Each flag says what to undo (R9.6, R10.4).
  $copying = $false; $copied = $false; $restored = $false; $movedOld = $false; $movedNew = $false; $ok = $false
  try {
    Remove-Tree $old  # left by an earlier switch
    $copying = $true
    Remove-Tree $backup  # R9.1: an older copy of this version is replaced
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

    [IO.Directory]::Move($install, $old)
    $movedOld = $true
    [IO.Directory]::Move($app, $install)
    $movedNew = $true

    # R10.5: the result first, since the new version reads it when it starts
    Write-Result $true $null
    Start-Grace
    $ok = $true
  } catch {
    $err = Get-Message $_
    # undo in reverse order, each step on its own so that one failure does not skip the rest
    if ($movedNew) { try { Move-Back $install $app } catch {} }
    if ($movedOld) { try { Move-Back $old $install } catch {} }
    # R10.4: the running version's data back from the copy just made, so a later try does not
    # overwrite the real backup with the target's data
    if ($restored) { try { Set-Data $backup } catch {} }
    if ($copying -and -not $copied) { try { Remove-Tree $backup } catch {} }
    try { Write-Result $false $err } catch {}
    Start-Grace
  }

  if ($ok) {
    # R9.5: keep the copies of the most recently backed-up versions. A folder without a valid
    # backup.json is not a copy: leave it alone.
    try {
      $copies = foreach ($d in Get-ChildItem -LiteralPath $backups -Directory) {
        try {
          $meta = Get-Content -LiteralPath (Join-Path $d.FullName 'backup.json') -Raw -Encoding UTF8 | ConvertFrom-Json
          New-Object PSObject -Property @{ Path = $d.FullName; SavedAt = [DateTime]$meta.savedAt }
        } catch {}
      }
      $copies | Sort-Object SavedAt -Descending | Select-Object -Skip $plan.keepBackups |
        ForEach-Object { Remove-Tree $_.Path }
    } catch {}
  }
} finally {
  Remove-Item -LiteralPath $switching -Force -ErrorAction SilentlyContinue
}
