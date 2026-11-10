// The probe of the 2026-11-10 GPUI P2 exit check (VL-26 to VL-29, the cursor rows). Built with the .NET Framework's
// csc.exe: `csc /nologo /out:uiprobe.exe /r:System.Drawing.dll /r:UIAutomationClient.dll /r:UIAutomationTypes.dll /r:WindowsBase.dll uiprobe.cs`.
// It sees both apps from outside, the same way: UI Automation names, screen pixels, the cursor, and the UI thread's
// answer to WM_NULL. Every command prints one JSON line.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Automation;

static class P {
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint f, int dx, int dy, int data, IntPtr extra);
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint f, IntPtr extra);
    [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr h);
    [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr h, IntPtr dc);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] static extern IntPtr SendMessageTimeout(IntPtr h, uint msg, IntPtr w, IntPtr l, uint flags, uint ms, out IntPtr r);
    [DllImport("user32.dll")] static extern bool GetCursorInfo(ref CURSORINFO ci);
    [DllImport("user32.dll")] static extern IntPtr LoadCursor(IntPtr inst, int id);
    [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleDC(IntPtr dc);
    [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleBitmap(IntPtr dc, int w, int h);
    [DllImport("gdi32.dll")] static extern IntPtr SelectObject(IntPtr dc, IntPtr o);
    [DllImport("gdi32.dll")] static extern bool BitBlt(IntPtr d, int x, int y, int w, int h, IntPtr s, int sx, int sy, uint op);
    [DllImport("gdi32.dll")] static extern int GetBitmapBits(IntPtr bmp, int n, byte[] bits);
    [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr o);
    [DllImport("gdi32.dll")] static extern bool DeleteDC(IntPtr dc);

    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [StructLayout(LayoutKind.Sequential)] struct RECT { public int l, t, r, b; }
    [StructLayout(LayoutKind.Sequential)] struct POINT { public int x, y; }
    [StructLayout(LayoutKind.Sequential)] struct CURSORINFO { public int cbSize, flags; public IntPtr hCursor; public POINT pt; }

    static long Now() { return DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(); }

    static IntPtr Window(int pid) {
        var p = Process.GetProcessById(pid);
        for (int i = 0; i < 100 && p.MainWindowHandle == IntPtr.Zero; i++) { Thread.Sleep(100); p.Refresh(); }
        return p.MainWindowHandle;
    }

    static AutomationElement Find(int pid, string name, bool prefix) {
        var root = AutomationElement.FromHandle(Window(pid));
        if (!prefix) return root.FindFirst(TreeScope.Descendants, new PropertyCondition(AutomationElement.NameProperty, name));
        foreach (AutomationElement e in root.FindAll(TreeScope.Descendants, Condition.TrueCondition))
            if (e.Current.Name.Contains(name)) return e;
        return null;
    }

    // A hash of the screen's pixels in the rectangle.
    static long Shot(int x, int y, int w, int h) {
        IntPtr s = GetDC(IntPtr.Zero), d = CreateCompatibleDC(s), b = CreateCompatibleBitmap(s, w, h), o = SelectObject(d, b);
        BitBlt(d, 0, 0, w, h, s, x, y, 0x00CC0020);
        var bits = new byte[w * h * 4];
        GetBitmapBits(b, bits.Length, bits);
        SelectObject(d, o); DeleteObject(b); DeleteDC(d); ReleaseDC(IntPtr.Zero, s);
        long hash = 1469598103934665603;
        for (int i = 0; i < bits.Length; i += 4) hash = (hash ^ (bits[i] | bits[i + 1] << 8 | bits[i + 2] << 16)) * 1099511628211;
        return hash;
    }

    // Milliseconds from the input to the first changed pixel in the rectangle, or -1 within the timeout.
    static long ToPaint(Action input, int x, int y, int w, int h, int timeout) {
        long before = Shot(x, y, w, h);
        var sw = Stopwatch.StartNew();
        input();
        while (sw.ElapsedMilliseconds < timeout) if (Shot(x, y, w, h) != before) return sw.ElapsedMilliseconds;
        return -1;
    }

    static void Click() { mouse_event(0x0002, 0, 0, 0, IntPtr.Zero); mouse_event(0x0004, 0, 0, 0, IntPtr.Zero); }
    static void Key(byte vk) { keybd_event(vk, 0, 0, IntPtr.Zero); keybd_event(vk, 0, 2, IntPtr.Zero); }

    static string CursorName() {
        var ci = new CURSORINFO { cbSize = Marshal.SizeOf(typeof(CURSORINFO)) };
        GetCursorInfo(ref ci);
        var names = new Dictionary<int, string> { { 32512, "arrow" }, { 32513, "text" }, { 32514, "wait" }, { 32515, "cross" },
            { 32649, "hand" }, { 32648, "not-allowed" }, { 32650, "app-starting" }, { 32644, "resize-we" }, { 32645, "resize-ns" },
            { 32646, "move" }, { 32642, "resize-nwse" }, { 32643, "resize-nesw" } };
        foreach (var kv in names) if (LoadCursor(IntPtr.Zero, kv.Key) == ci.hCursor) return kv.Value;
        return "other:" + ci.hCursor.ToInt64();
    }

    static int Main(string[] a) {
        SetProcessDPIAware();
        try {
            switch (a[0]) {
                case "rect": { // rect <pid> <name> [prefix]
                    var e = Find(int.Parse(a[1]), a[2], a.Length > 3);
                    if (e == null) { Console.WriteLine("null"); return 1; }
                    var r = e.Current.BoundingRectangle;
                    Console.WriteLine("{{\"x\":{0},\"y\":{1},\"w\":{2},\"h\":{3},\"name\":\"{4}\"}}", (int)r.X, (int)r.Y, (int)r.Width, (int)r.Height,
                        e.Current.Name.Replace("\\", "\\\\").Replace("\"", "\\\""));
                    return 0;
                }
                case "waitname": { // waitname <pid> <name> <timeoutMs> [prefix]: when the name first shows
                    int pid = int.Parse(a[1]); var sw = Stopwatch.StartNew();
                    while (sw.ElapsedMilliseconds < int.Parse(a[3])) {
                        try { if (Find(pid, a[2], a.Length > 4) != null) { Console.WriteLine("{{\"at\":{0}}}", Now()); return 0; } } catch (Exception) { }
                        Thread.Sleep(25);
                    }
                    Console.WriteLine("{\"at\":null}"); return 1;
                }
                case "clickpaint": { // clickpaint <x> <y> <rx> <ry> <rw> <rh>
                    int x = int.Parse(a[1]), y = int.Parse(a[2]);
                    SetCursorPos(x, y); Thread.Sleep(1500); // hover and any tooltip settle first
                    long ms = ToPaint(Click, int.Parse(a[3]), int.Parse(a[4]), int.Parse(a[5]), int.Parse(a[6]), 10000);
                    Console.WriteLine("{{\"ms\":{0}}}", ms); return 0;
                }
                case "keypaint": { // keypaint <vk> <rx> <ry> <rw> <rh>
                    byte vk = byte.Parse(a[1]);
                    long ms = ToPaint(() => Key(vk), int.Parse(a[2]), int.Parse(a[3]), int.Parse(a[4]), int.Parse(a[5]), 10000);
                    Console.WriteLine("{{\"ms\":{0}}}", ms); return 0;
                }
                case "dump": { // dump <pid>: every named element, one per line
                    foreach (AutomationElement e in AutomationElement.FromHandle(Window(int.Parse(a[1]))).FindAll(TreeScope.Descendants, Condition.TrueCondition)) {
                        var r = e.Current.BoundingRectangle;
                        Console.WriteLine("{0}\t{1}\t{2},{3},{4},{5}", e.Current.ControlType.ProgrammaticName, e.Current.Name, r.X, r.Y, r.Width, r.Height);
                    }
                    return 0;
                }
                case "png": { // png <x> <y> <w> <h> <file>: the screen's pixels in the rectangle
                    int w = int.Parse(a[3]), h = int.Parse(a[4]);
                    using (var bmp = new System.Drawing.Bitmap(w, h))
                    using (var g = System.Drawing.Graphics.FromImage(bmp)) {
                        g.CopyFromScreen(int.Parse(a[1]), int.Parse(a[2]), 0, 0, new System.Drawing.Size(w, h));
                        bmp.Save(a[5], System.Drawing.Imaging.ImageFormat.Png);
                    }
                    Console.WriteLine("{}"); return 0;
                }
                case "frames": { // frames <x> <y> <w> <h> <ms>: each distinct frame's mean brightness (0-255) and white share
                    int x = int.Parse(a[1]), y = int.Parse(a[2]), w = int.Parse(a[3]), h = int.Parse(a[4]);
                    var sw = Stopwatch.StartNew(); var outl = new List<string>(); string last = "";
                    IntPtr s = GetDC(IntPtr.Zero), d = CreateCompatibleDC(s), b = CreateCompatibleBitmap(s, w, h), o = SelectObject(d, b);
                    var bits = new byte[w * h * 4];
                    while (sw.ElapsedMilliseconds < int.Parse(a[5])) {
                        BitBlt(d, 0, 0, w, h, s, x, y, 0x00CC0020); GetBitmapBits(b, bits.Length, bits);
                        long sum = 0, white = 0, n = 0;
                        for (int i = 0; i < bits.Length; i += 64) { int l = (bits[i] + bits[i + 1] + bits[i + 2]) / 3; sum += l; if (l > 240) white++; n++; }
                        string cur = string.Format("{0},{1:0.00}", sum / n, (double)white / n);
                        if (cur != last) { outl.Add(string.Format("[{0},{1}]", sw.ElapsedMilliseconds, cur)); last = cur; }
                    }
                    SelectObject(d, o); DeleteObject(b); DeleteDC(d); ReleaseDC(IntPtr.Zero, s);
                    Console.WriteLine("{{\"frames\":[{0}]}}", string.Join(",", outl)); return 0;
                }
                case "keys": { // keys <vk>[+<vk>...]: a chord, pressed in order and released in reverse
                    var vks = a[1].Split('+').Select(byte.Parse).ToArray();
                    foreach (var k in vks) keybd_event(k, 0, 0, IntPtr.Zero);
                    foreach (var k in vks.Reverse()) keybd_event(k, 0, 2, IntPtr.Zero);
                    Console.WriteLine("{}"); return 0;
                }
                case "minimise": { ShowWindow(Window(int.Parse(a[1])), 6); Console.WriteLine("{}"); return 0; }
                case "state": { // state <pid>: minimised, maximised, in front
                    IntPtr h = Window(int.Parse(a[1]));
                    Console.WriteLine("{{\"minimised\":{0},\"maximised\":{1},\"front\":{2}}}", IsIconic(h) ? "true" : "false",
                        IsZoomed(h) ? "true" : "false", GetForegroundWindow() == h ? "true" : "false"); return 0;
                }
                case "winrect": { // winrect <pid>: the main window's rectangle
                    RECT r; GetWindowRect(Window(int.Parse(a[1])), out r);
                    Console.WriteLine("{{\"x\":{0},\"y\":{1},\"w\":{2},\"h\":{3}}}", r.l, r.t, r.r - r.l, r.b - r.t); return 0;
                }
                case "drag": { // drag <x1> <y1> <x2> <y2>: press, move in 20 steps, release
                    int x1 = int.Parse(a[1]), y1 = int.Parse(a[2]), x2 = int.Parse(a[3]), y2 = int.Parse(a[4]);
                    SetCursorPos(x1, y1); Thread.Sleep(300); mouse_event(0x0002, 0, 0, 0, IntPtr.Zero); Thread.Sleep(100);
                    for (int i = 1; i <= 20; i++) { SetCursorPos(x1 + (x2 - x1) * i / 20, y1 + (y2 - y1) * i / 20); mouse_event(0x0001, 0, 0, 0, IntPtr.Zero); Thread.Sleep(20); }
                    mouse_event(0x0004, 0, 0, 0, IntPtr.Zero); Console.WriteLine("{}"); return 0;
                }
                case "move": { SetCursorPos(int.Parse(a[1]), int.Parse(a[2])); Console.WriteLine("{}"); return 0; }
                case "click": { SetCursorPos(int.Parse(a[1]), int.Parse(a[2])); Thread.Sleep(200); Click(); Console.WriteLine("{}"); return 0; }
                case "front": { Console.WriteLine("{{\"ok\":{0}}}", SetForegroundWindow(Window(int.Parse(a[1]))) ? "true" : "false"); return 0; }
                case "cursor": { // cursor <x> <y>: the cursor's shape over a point
                    SetCursorPos(int.Parse(a[1]), int.Parse(a[2])); Thread.Sleep(200); SetCursorPos(int.Parse(a[1]) + 1, int.Parse(a[2])); Thread.Sleep(400); // the nudge: a second move, as a hand never stays still
                    Console.WriteLine("{{\"cursor\":\"{0}\"}}", CursorName()); return 0;
                }
                case "wheel": { // wheel <x> <y> <notches> <everyMs>
                    SetCursorPos(int.Parse(a[1]), int.Parse(a[2])); Thread.Sleep(100);
                    for (int i = 0; i < int.Parse(a[3]); i++) { mouse_event(0x0800, 0, 0, -120, IntPtr.Zero); Thread.Sleep(int.Parse(a[4])); }
                    Console.WriteLine("{}"); return 0;
                }
                case "ping": { // ping <pid> <stopFile>: the UI thread's answers to WM_NULL until the stop file shows
                    IntPtr h = Window(int.Parse(a[1])); var lat = new List<long>(); var slow = new List<string>();
                    while (!File.Exists(a[2])) {
                        var sw = Stopwatch.StartNew(); IntPtr r;
                        SendMessageTimeout(h, 0, IntPtr.Zero, IntPtr.Zero, 0, 20000, out r);
                        long ms = sw.ElapsedMilliseconds; lat.Add(ms);
                        if (ms >= 50) slow.Add(string.Format("[{0},{1}]", Now() - ms, ms));
                        Thread.Sleep(5);
                    }
                    Console.WriteLine("{{\"n\":{0},\"max\":{1},\"over50\":{2},\"over100\":{3},\"over500\":{4},\"slow\":[{5}]}}", lat.Count,
                        lat.Count > 0 ? lat.Max() : 0, lat.Count(v => v >= 50), lat.Count(v => v >= 100), lat.Count(v => v >= 500), string.Join(",", slow));
                    return 0;
                }
            }
            Console.Error.WriteLine("unknown command"); return 2;
        } catch (Exception e) { Console.Error.WriteLine(e); return 3; }
    }
}
