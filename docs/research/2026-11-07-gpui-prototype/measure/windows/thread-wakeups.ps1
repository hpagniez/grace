# Prints one JSON line: every thread of a process with its cumulative context switches, CPU, name and start module.
# Run twice and subtract to get wake-ups per thread. Needs no admin rights. See ../../results/windows-idle-patch.md.
param([int]$ProcessId)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class T {
  [DllImport("kernel32.dll")] public static extern IntPtr OpenThread(uint access, bool inherit, uint id);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode)] public static extern int GetThreadDescription(IntPtr h, out IntPtr desc);
  [DllImport("kernel32.dll")] public static extern IntPtr LocalFree(IntPtr p);
  [DllImport("ntdll.dll")] public static extern int NtQueryInformationThread(IntPtr h, int cls, out IntPtr info, int size, IntPtr ret);
}
'@
$mods = try { (Get-Process -Id $ProcessId).Modules | ForEach-Object { [pscustomobject]@{ n = $_.ModuleName; b = $_.BaseAddress.ToInt64(); e = $_.BaseAddress.ToInt64() + $_.ModuleMemorySize } } } catch { @() }
$rows = Get-CimInstance -Query "SELECT IDThread,ContextSwitchesPersec,PercentProcessorTime,ThreadWaitReason,PriorityCurrent FROM Win32_PerfRawData_PerfProc_Thread WHERE IDProcess=$ProcessId" | ForEach-Object {
  $h = [T]::OpenThread(0x0040 -bor 0x0800, $false, [uint32]$_.IDThread) # QUERY_INFORMATION | QUERY_LIMITED
  $name = ''; $start = ''
  if ($h -ne [IntPtr]::Zero) {
    $p = [IntPtr]::Zero
    if ([T]::GetThreadDescription($h, [ref]$p) -eq 0 -and $p -ne [IntPtr]::Zero) { $name = [Runtime.InteropServices.Marshal]::PtrToStringUni($p); [void][T]::LocalFree($p) }
    $a = [IntPtr]::Zero
    if ([T]::NtQueryInformationThread($h, 9, [ref]$a, [IntPtr]::Size, [IntPtr]::Zero) -eq 0) {
      $v = $a.ToInt64(); $m = $mods | Where-Object { $v -ge $_.b -and $v -lt $_.e } | Select-Object -First 1
      $start = if ($m) { '{0}+0x{1:x}' -f $m.n, ($v - $m.b) } else { '0x{0:x}' -f $v }
    }
    [void][T]::CloseHandle($h)
  }
  [pscustomobject]@{ id = [int]$_.IDThread; cs = [long]$_.ContextSwitchesPersec; cpu100ns = [long]$_.PercentProcessorTime; wait = [int]$_.ThreadWaitReason; prio = [int]$_.PriorityCurrent; name = $name; start = $start }
}
ConvertTo-Json -Compress -Depth 3 -InputObject @($rows)
