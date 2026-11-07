# Window helpers for measure-windows.mjs: the two things the measurement needs from Win32 that Grace's bench
# (bench/system.mjs) does not offer. Run by the measurement as
#   powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File window-tools.ps1 -Action <action> -ProcessId <pid> ...
# and prints one JSON line.
#
#   move   Moves the process's main window to the middle of the first screen that is not the main one (by device
#          name, as bench/system.mjs windowSpot does), so the prototype sits where the bench puts Grace. With no
#          second screen it leaves the window and says so. The size does not change.
#   wheel  Brings the window to the front, puts the pointer over it at (OffsetX, OffsetY) from its top left corner
#          and turns the mouse wheel Notches times, one every DelayMs: the input of bench/probes.mjs scrollTree, sent
#          by the operating system, so every app gets it the same way (a real WM_MOUSEWHEEL, not CDP). With StartAt
#          (ms since the epoch) it does its slow setup first and sends the first notch at that time; lateMs says by
#          how much it missed.
#   displays  Prints the screens (name, size, position) and the display adapters (name, driver, refresh rate). No ProcessId.
#
# Untested on a real machine (written on Linux): see README.md.
param(
  [Parameter(Mandatory = $true)][ValidateSet('move', 'wheel', 'displays')][string]$Action,
  [int]$ProcessId = 0,
  [int]$Notches = 60,
  [int]$DelayMs = 16,
  [int]$OffsetX = 250,
  [int]$OffsetY = 400,
  [long]$StartAt = 0,
  [switch]$Up
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Win {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, int data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int command);
}
'@

# Physical pixels everywhere: GetWindowRect, SetWindowPos, SetCursorPos and WinForms' Screen then agree on one scale.
# -4 is DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 (Windows 10 1703 and later).
[void][Win]::SetProcessDpiAwarenessContext([IntPtr]::new(-4))
Add-Type -AssemblyName System.Windows.Forms

function Get-MainWindow([int]$id) {
  $handle = (Get-Process -Id $id).MainWindowHandle
  if ($handle -eq [IntPtr]::Zero) { throw "process $id has no window" }
  $handle
}

switch ($Action) {
  'displays' {
    $screens = @([System.Windows.Forms.Screen]::AllScreens | ForEach-Object {
      [pscustomobject]@{ name = $_.DeviceName; primary = $_.Primary; width = $_.Bounds.Width; height = $_.Bounds.Height; x = $_.Bounds.X; y = $_.Bounds.Y }
    })
    $cards = @(Get-CimInstance Win32_VideoController | ForEach-Object {
      [pscustomobject]@{ name = $_.Name; driver = $_.DriverVersion; hz = $_.CurrentRefreshRate; width = $_.CurrentHorizontalResolution; height = $_.CurrentVerticalResolution }
    })
    ConvertTo-Json -Compress -Depth 4 -InputObject ([pscustomobject]@{ screens = $screens; adapters = $cards })
  }

  'move' {
    $handle = Get-MainWindow $ProcessId
    $other = @([System.Windows.Forms.Screen]::AllScreens | Where-Object { -not $_.Primary } | Sort-Object DeviceName) | Select-Object -First 1
    if (-not $other) {
      ConvertTo-Json -Compress -InputObject ([pscustomobject]@{ moved = $false; reason = 'no screen besides the main one' })
      return
    }
    $rect = New-Object Win+RECT
    [void][Win]::GetWindowRect($handle, [ref]$rect)
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $area = $other.WorkingArea
    $x = $area.X + [Math]::Max(0, [int](($area.Width - $width) / 2))
    $y = $area.Y + [Math]::Max(0, [int](($area.Height - $height) / 2))
    # SWP_NOSIZE 0x1, SWP_NOZORDER 0x4, SWP_NOACTIVATE 0x10
    if (-not [Win]::SetWindowPos($handle, [IntPtr]::Zero, $x, $y, 0, 0, 0x15)) { throw 'SetWindowPos failed' }
    ConvertTo-Json -Compress -InputObject ([pscustomobject]@{ moved = $true; screen = $other.DeviceName; x = $x; y = $y })
  }

  'wheel' {
    $handle = Get-MainWindow $ProcessId
    if ([Win]::IsIconic($handle)) { [void][Win]::ShowWindow($handle, 9) } # SW_RESTORE
    # Windows refuses SetForegroundWindow from a background process unless a key went down first: tap Alt.
    [Win]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
    [Win]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [void][Win]::SetForegroundWindow($handle)
    Start-Sleep -Milliseconds 300
    $rect = New-Object Win+RECT
    [void][Win]::GetWindowRect($handle, [ref]$rect)
    if (-not [Win]::SetCursorPos($rect.Left + $OffsetX, $rect.Top + $OffsetY)) { throw 'SetCursorPos failed' }
    Start-Sleep -Milliseconds 100
    $late = 0
    if ($StartAt -gt 0) {
      $wait = $StartAt - [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
      if ($wait -gt 0) { Start-Sleep -Milliseconds ([int]$wait) } else { $late = -$wait }
    }
    $delta = if ($Up) { 120 } else { -120 } # WHEEL_DELTA is 120 per notch; negative turns toward the user (down)
    $clock = [Diagnostics.Stopwatch]::StartNew()
    for ($i = 0; $i -lt $Notches; $i++) {
      [Win]::mouse_event(0x0800, 0, 0, $delta, [UIntPtr]::Zero) # MOUSEEVENTF_WHEEL
      Start-Sleep -Milliseconds $DelayMs
    }
    ConvertTo-Json -Compress -InputObject ([pscustomobject]@{ ms = $clock.ElapsedMilliseconds; lateMs = $late; notches = $Notches; x = $rect.Left + $OffsetX; y = $rect.Top + $OffsetY })
  }
}
