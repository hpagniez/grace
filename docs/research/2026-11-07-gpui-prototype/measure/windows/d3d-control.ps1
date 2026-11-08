# Windowless control for ../../results/windows-idle-patch.md: holds a bare D3D11 device for 60 s. Measure it with thread-wakeups.ps1.
# Flags: 0x20 BGRA_SUPPORT (what GPUI uses), 0x8 PREVENT_INTERNAL_THREADING_OPTIMIZATIONS, 0x1 SINGLETHREADED.
param([uint32]$Flags)
# A bare D3D11 device on the default hardware adapter, no window. Holds it until killed.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class D {
  [DllImport("d3d11.dll")] public static extern int D3D11CreateDevice(IntPtr adapter, int type, IntPtr sw, uint flags, IntPtr levels, uint n, uint sdk, out IntPtr dev, out int level, out IntPtr ctx);
}
'@
$dev = [IntPtr]::Zero; $ctx = [IntPtr]::Zero; $lvl = 0
$hr = [D]::D3D11CreateDevice([IntPtr]::Zero, 1, [IntPtr]::Zero, $Flags, [IntPtr]::Zero, 0, 7, [ref]$dev, [ref]$lvl, [ref]$ctx)
[Console]::Out.WriteLine("hr=$hr level=$lvl")
[Console]::Out.Flush()
Start-Sleep -Seconds 60
