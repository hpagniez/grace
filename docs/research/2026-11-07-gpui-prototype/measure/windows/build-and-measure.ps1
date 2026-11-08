<#
.SYNOPSIS
  Builds the GPUI prototype for Windows and measures it against the packaged Grace on this machine (issue #267).

.DESCRIPTION
  One command, from a Grace checkout that has this folder:
    powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1

  It checks for Node, Rust (rustup, the MSVC target), the MSVC build tools and the Windows SDK, and says how to install
  what is missing. Then it builds the prototype (release, and the empty-window "floor" example), runs `npm run vendor` and
  `npm run package` in the Grace checkout when they have not been run, and calls measure-windows.mjs, which measures both
  apps the way bench/bench.mjs does and writes results\windows-<Label>.json and .md.

  Run it once with -Quick first (a few minutes, one round): it proves every step works before the real run, which takes
  about an hour. Do not touch the machine during a run; the windows open on the second screen when there is one.

  For the run without a GPU, disable the display adapter in Device Manager (it falls back to the Microsoft Basic Display
  Adapter), then run again with -Label no-gpu. README.md in this folder has the steps.

.EXAMPLE
  .\build-and-measure.ps1 -Quick

.EXAMPLE
  .\build-and-measure.ps1 -Label gpu

.EXAMPLE
  .\build-and-measure.ps1 -Label no-gpu -SkipBuild -SkipPackage
#>
[CmdletBinding()]
param(
  # The Grace checkout. Default: the one this script is in.
  [string]$Grace,
  # The folder `npm run vendor` fills (jre\bin\java.exe, karate.jar). Default: <Grace>\vendor.
  [string]$Vendor,
  # The packaged Grace. Default: <Grace>\out\Grace-win32-x64\Grace.exe, made by `npm run package` when missing.
  [string]$GraceExe,
  # Cargo's target folder. Default: <Grace>\out\gpui-prototype-target (out\ is git-ignored).
  [string]$TargetDir,
  # Where the results go. Default: the prototype's results\ folder.
  [string]$Out,
  # Names the run in the file names: gpu, or no-gpu for the run with the GPU off.
  [string]$Label = 'gpu',
  [int]$Rounds = 5,
  [int]$FloorRounds = 3,
  # One round, a short idle, no 10,000-test project, one floor round: checks that every step works, in a few minutes.
  [switch]$Quick,
  [switch]$SkipBuild,
  [switch]$SkipPackage,
  [switch]$SkipLarge,
  [switch]$SkipFluid,
  [switch]$SkipFloor,
  # Leave the prototype's window where it opens (0, 0 of the main screen) instead of moving it to Grace's screen.
  [switch]$KeepGpuiWindow,
  # Check the prerequisites and stop.
  [switch]$CheckOnly
)

$ErrorActionPreference = 'Stop'

function Say([string]$Message) { Write-Host "==> $Message" -ForegroundColor Cyan }
function Warn([string]$Message) { Write-Host "!!  $Message" -ForegroundColor Yellow }
function Fail([string]$Message) {
  Write-Host ''
  Write-Host "XX  $Message" -ForegroundColor Red
  exit 1
}
function Have([string]$Name) { [bool](Get-Command $Name -ErrorAction SilentlyContinue) }

# ---- where things are ----

$scriptDir = $PSScriptRoot
$proto = (Resolve-Path (Join-Path $scriptDir '..\..')).Path
if (-not (Test-Path (Join-Path $proto 'Cargo.toml'))) { Fail "No Cargo.toml in $proto. Run this script from its place in the Grace repository: docs\research\2026-11-07-gpui-prototype\measure\windows." }
if (-not $Grace) { $Grace = (Resolve-Path (Join-Path $proto '..\..\..')).Path }
$Grace = (Resolve-Path $Grace).Path
if (-not (Test-Path (Join-Path $Grace 'bench\system.mjs'))) { Fail "$Grace is not a Grace checkout (no bench\system.mjs). Pass -Grace <path>." }
$vendorGiven = [bool]$Vendor
if (-not $Vendor) { $Vendor = Join-Path $Grace 'vendor' }
if (-not $GraceExe) { $GraceExe = Join-Path $Grace 'out\Grace-win32-x64\Grace.exe' }
if (-not $TargetDir) { $TargetDir = Join-Path $Grace 'out\gpui-prototype-target' }
if (-not $Out) { $Out = Join-Path $proto 'results' }
$java = Join-Path $Vendor 'jre\bin\java.exe'
$jar = Join-Path $Vendor 'karate.jar'

$settleSeconds = 20
$idleSeconds = 60
if ($Quick) {
  $Rounds = 1
  $FloorRounds = 1
  $settleSeconds = 5
  $idleSeconds = 10
  $SkipLarge = $true
  $Label = "$Label-quick"
}

Say "Grace checkout: $Grace"
Say "Prototype:      $proto"
Say "Label: $Label, rounds: $Rounds$(if ($Quick) { ' (quick)' })"

# ---- 1. what the machine needs ----

Say 'Checking the prerequisites'
$problems = New-Object System.Collections.Generic.List[string]

if ($env:OS -ne 'Windows_NT') { Fail 'This script is for Windows 11.' }
if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { Warn "The processor architecture is $($env:PROCESSOR_ARCHITECTURE); the reference machine is x64 and this script builds for x86_64." }

# Node 24, as Grace itself needs
if (-not (Have 'node')) {
  $problems.Add("Node.js is missing. Install it:  winget install OpenJS.NodeJS   (Grace needs Node 24), then open a new terminal.")
} else {
  $nodeVersion = (& node --version)
  if ($nodeVersion -notmatch '^v(\d+)\.' -or [int]$Matches[1] -lt 24) { $problems.Add("Node $nodeVersion is too old; Grace needs Node 24. Update it:  winget upgrade OpenJS.NodeJS") }
}
if (-not (Have 'npm')) { $problems.Add('npm is missing (it comes with Node.js).') }

# Rust, on the MSVC target
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (-not (Have 'cargo') -and (Test-Path (Join-Path $cargoBin 'cargo.exe'))) { $env:PATH = "$cargoBin;$env:PATH" }
if (-not (Have 'cargo')) {
  $problems.Add("Rust is missing. Install it:  winget install Rustlang.Rustup   then open a new terminal and run:  rustup default stable-x86_64-pc-windows-msvc")
} else {
  $rustc = (& rustc --version)
  if ($rustc -notmatch 'rustc (\d+)\.(\d+)') { $problems.Add("Could not read the Rust version from '$rustc'.") }
  elseif ([int]$Matches[1] -eq 1 -and [int]$Matches[2] -lt 88) { $problems.Add("$rustc is too old (the prototype and GPUI need 1.88 or newer; it was built with 1.97). Update it:  rustup update stable") }
  $host_ = (& rustc -vV | Select-String '^host:').ToString()
  if ($host_ -notmatch 'x86_64-pc-windows-msvc') { $problems.Add("Rust's default target is '$host_', not x86_64-pc-windows-msvc. Run:  rustup default stable-x86_64-pc-windows-msvc") }
}

# MSVC build tools (the linker) and the Windows SDK (rc.exe for the app's manifest, fxc.exe for GPUI's shaders)
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$msvc = $null
if (Test-Path $vswhere) { $msvc = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) }
if (-not $msvc) {
  $problems.Add("The MSVC build tools are missing. Install them (about 3 GB):`n      winget install Microsoft.VisualStudio.2022.BuildTools --override `"--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`"`n    That is the 'Desktop development with C++' workload with the Windows 11 SDK; then open a new terminal.")
}
$kitBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
function Find-SdkTool([string]$Name) {
  if (-not (Test-Path $kitBin)) { return $null }
  Get-ChildItem $kitBin -Directory |
    Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
    Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1
}
$fxc = $null
if ($env:GPUI_FXC_PATH -and (Test-Path $env:GPUI_FXC_PATH)) { $fxc = $env:GPUI_FXC_PATH } else { $fxc = Find-SdkTool 'fxc.exe' }
$rc = Find-SdkTool 'rc.exe'
if (-not $fxc -or -not $rc) {
  $problems.Add("The Windows SDK is missing ($(if (-not $fxc) { 'fxc.exe' }) $(if (-not $rc) { 'rc.exe' }) not found under $kitBin). Add the 'Windows 11 SDK' component in the Visual Studio Installer (Modify the Build Tools), or install the workload with the winget line above.")
}

# Grace's own side: node_modules, the vendor folder, the packaged exe
if (-not (Test-Path (Join-Path $Grace 'node_modules\@playwright\test'))) { Warn 'Grace has no node_modules yet: `npm install` runs below.' }
$graceNeedsVendor = -not ((Test-Path $java) -and (Test-Path $jar))
if ($graceNeedsVendor -and $vendorGiven) { $problems.Add("-Vendor $Vendor has no jre\bin\java.exe or karate.jar. Run 'npm run vendor' in the Grace checkout, or point -Vendor at its vendor folder.") }

# The machine itself: leftovers and power
$left = @(Get-Process -Name 'Grace', 'grace-gpui', 'java', 'electron' -ErrorAction SilentlyContinue)
if ($left.Count) { Warn "Running now: $((($left | ForEach-Object { $_.ProcessName }) | Sort-Object -Unique) -join ', '). Close them: they skew the idle and CPU figures, and a stray java.exe is a leak." }
try {
  $battery = @(Get-CimInstance Win32_Battery -ErrorAction Stop)
  if ($battery.Count -and $battery[0].BatteryStatus -eq 1) { Warn 'The machine runs on battery. Plug it in: the numbers are only comparable on mains power.' }
} catch { }
Say ("Power plan: " + ((& powercfg /getactivescheme) -replace '^.*:\s*', ''))

if ($problems.Count) {
  Write-Host ''
  Write-Host 'Missing before this can run:' -ForegroundColor Red
  $n = 1
  foreach ($p in $problems) { Write-Host "  $n. $p" -ForegroundColor Red; $n++ }
  Write-Host ''
  Write-Host 'Install what is listed, open a new terminal (so the PATH is refreshed) and run this script again.' -ForegroundColor Red
  exit 1
}
Say "Windows SDK tools: fxc $fxc"
$env:GPUI_FXC_PATH = $fxc # GPUI's build script compiles its shaders with it
if ($CheckOnly) { Say 'All prerequisites are there.'; exit 0 }

# ---- 2. Grace: npm install, vendor, package ----

Push-Location $Grace
try {
  if (-not (Test-Path (Join-Path $Grace 'node_modules\@playwright\test'))) {
    Say 'npm install (Grace)'
    & npm install
    if ($LASTEXITCODE -ne 0) { Fail 'npm install failed.' }
  }
  if ($graceNeedsVendor) {
    Say 'npm run vendor (downloads the JRE and the Karate jar into vendor\)'
    & npm run vendor
    if ($LASTEXITCODE -ne 0) { Fail 'npm run vendor failed.' }
    if (-not ((Test-Path $java) -and (Test-Path $jar))) { Fail "npm run vendor ran, but $java or $jar is still missing." }
  }
  if (-not (Test-Path $GraceExe)) {
    if ($SkipPackage) { Fail "$GraceExe does not exist and -SkipPackage was given." }
    Say 'npm run package (the packaged Grace to measure)'
    & npm run package
    if ($LASTEXITCODE -ne 0) { Fail 'npm run package failed.' }
    if (-not (Test-Path $GraceExe)) { Fail "npm run package ran, but $GraceExe does not exist. Pass -GraceExe <path> if it went elsewhere." }
  } else {
    Say "Using the packaged Grace at $GraceExe (delete out\Grace-win32-x64 to repackage)"
  }
  # electron's binary comes on first use, not with npm install, and the Electron floor runs it
  if (-not (Test-Path (Join-Path $Grace 'node_modules\electron\dist\electron.exe'))) {
    Say 'npx electron --version (downloads the Electron binary for the empty-window floor)'
    & npx electron --version
    if ($LASTEXITCODE -ne 0) { Warn 'npx electron --version failed: the Electron floor is skipped.' }
  }
} finally { Pop-Location }

# ---- 3. the prototype ----

$gpuiExe = Join-Path $TargetDir 'release\grace-gpui.exe'
$floorExe = Join-Path $TargetDir 'release\examples\floor.exe'
if ($SkipBuild) {
  if (-not (Test-Path $gpuiExe)) { Fail "-SkipBuild was given, but $gpuiExe does not exist." }
} else {
  Say "cargo build --release (first build: 5 to 15 minutes; target folder $TargetDir)"
  $env:CARGO_TARGET_DIR = $TargetDir
  $clock = [Diagnostics.Stopwatch]::StartNew()
  Push-Location $proto
  try {
    & cargo build --release
    if ($LASTEXITCODE -ne 0) { Fail 'cargo build --release failed. The first error above says which prerequisite or crate; see README.md in this folder.' }
    & cargo build --release --example floor
    if ($LASTEXITCODE -ne 0) { Fail 'cargo build --release --example floor failed.' }
  } finally { Pop-Location }
  Say ("Built in {0:N0} s: {1} ({2:N1} MB)" -f $clock.Elapsed.TotalSeconds, $gpuiExe, ((Get-Item $gpuiExe).Length / 1e6))
}
if (-not (Test-Path $floorExe)) { Warn "No ${floorExe}: the GPUI floor is skipped."; $floorExe = $null }

# ---- 4. measure ----

New-Item -ItemType Directory -Force -Path $Out | Out-Null
$transcript = Join-Path $Out "windows-$Label.log"
try { Start-Transcript -Path $transcript -Force | Out-Null } catch { Warn "No transcript: $($_.Exception.Message)" }

$nodeArgs = @(
  '--no-warnings', (Join-Path $scriptDir 'measure-windows.mjs'),
  '--grace', $Grace, '--grace-exe', $GraceExe, '--gpui', $gpuiExe, '--java', $java, '--jar', $jar,
  '--rounds', "$Rounds", '--floor-rounds', "$FloorRounds", '--settle-s', "$settleSeconds", '--idle-s', "$idleSeconds",
  '--label', $Label, '--out', $Out
)
if ($floorExe) { $nodeArgs += @('--gpui-floor', $floorExe) }
if ($SkipLarge) { $nodeArgs += '--skip-large' }
if ($SkipFluid) { $nodeArgs += '--skip-fluid' }
if ($SkipFloor) { $nodeArgs += '--skip-floor' }
if ($KeepGpuiWindow) { $nodeArgs += '--keep-gpui-window' }

$minutes = if ($Quick) { '10 to 15' } else { 'about 90' }
Say "Measuring ($minutes minutes). Do not touch the machine until it is done."
Push-Location $Grace
try {
  & node @nodeArgs
  $code = $LASTEXITCODE
} finally { Pop-Location }

try { Stop-Transcript | Out-Null } catch { }
Write-Host ''
if ($code -eq 0) { Say "Done. Results: $Out\windows-$Label.json and .md" }
else { Warn "The measurement ended with exit code $code (a phase failed: the .md lists which). Results so far: $Out\windows-$Label.json and .md" }
exit $code
