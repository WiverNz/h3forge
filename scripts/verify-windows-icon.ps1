param([string]$Path = "$PSScriptRoot/../.local/release/H3Forge.exe")
$ErrorActionPreference = 'Stop'
# Mirror Chromium's util_read_icon.cc: map without executing, enumerate the
# first icon group, then LoadImage at download-list sizes and display scales.
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
public static class DownloadIconCheck {
    private delegate bool EnumName(IntPtr module, IntPtr type, IntPtr name, IntPtr param);
    [DllImport("kernel32.dll", SetLastError=true)]
    private static extern IntPtr CreateFileMapping(IntPtr file, IntPtr attrs, uint protect, uint high, uint low, string name);
    [DllImport("kernel32.dll", SetLastError=true)]
    private static extern IntPtr MapViewOfFile(IntPtr mapping, uint access, uint high, uint low, UIntPtr size);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    private static extern bool EnumResourceNames(IntPtr module, IntPtr type, EnumName callback, IntPtr param);
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    private static extern IntPtr LoadImage(IntPtr module, IntPtr name, uint type, int width, int height, uint flags);
    [DllImport("user32.dll")] private static extern bool DestroyIcon(IntPtr icon);
    [DllImport("kernel32.dll")] private static extern bool UnmapViewOfFile(IntPtr view);
    [DllImport("kernel32.dll")] private static extern bool CloseHandle(IntPtr handle);
    public static void Verify(string path) {
        using (var file = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete)) {
            IntPtr mapping = CreateFileMapping(file.SafeFileHandle.DangerousGetHandle(), IntPtr.Zero, 0x11000002, 0, 0, null);
            if (mapping == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot map EXE image");
            IntPtr view = IntPtr.Zero;
            try {
                view = MapViewOfFile(mapping, 4, 0, 0, UIntPtr.Zero);
                if (view == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
                bool found = false;
                Exception failure = null;
                EnumName callback = delegate(IntPtr module, IntPtr type, IntPtr name, IntPtr param) {
                    found = true;
                    foreach (int size in new int[] {16, 20, 24, 32, 40, 48, 64, 72, 96}) {
                        IntPtr icon = LoadImage(module, name, 1, size, size, 0);
                        if (icon == IntPtr.Zero) {
                            failure = new Win32Exception(Marshal.GetLastWin32Error(), "Download icon extraction failed at " + size + " px");
                            break;
                        }
                        DestroyIcon(icon);
                    }
                    return false;
                };
                EnumResourceNames(view, new IntPtr(14), callback, IntPtr.Zero);
                GC.KeepAlive(callback);
                if (failure != null) throw failure;
                if (!found) throw new Exception("EXE has no icon group");
            } finally {
                if (view != IntPtr.Zero) UnmapViewOfFile(view);
                CloseHandle(mapping);
            }
        }
    }
}
'@
[DownloadIconCheck]::Verify((Resolve-Path -LiteralPath $Path).Path)
Write-Host 'Windows download icon extraction passed at 16-96 px.'
