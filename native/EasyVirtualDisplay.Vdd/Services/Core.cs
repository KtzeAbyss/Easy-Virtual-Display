using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Interop;

namespace EasyVirtualDisplay.Vdd.Services;

internal static unsafe class Core
{
    public const string NAME = "Parsec Virtual Display";

    public const string DISPLAY_ID = "PSCCDD0";
    public const string DISPLAY_NAME = "ParsecVDA";

    public const string ADAPTER = "Parsec Virtual Display Adapter";
    public const string ADAPTER_GUID = "{00b41627-04c4-429e-a26e-0265cf50c8fa}";

    public const string HARDWARE_ID = @"Root\Parsec\VDA";
    public const string CLASS_GUID = "{4d36e968-e325-11ce-bfc1-08002be10318}";

    public static int MAX_DISPLAYS => 8;

    public static bool OpenHandle(out IntPtr vdd)
    {
        if (Device.OpenHandle(ADAPTER_GUID, out vdd))
        {
            _ = Update(vdd);
            return true;
        }

        return false;
    }

    public static void CloseHandle(IntPtr vdd)
    {
        Device.CloseHandle(vdd);
    }

    public static List<Display> GetDisplays(out bool noMonitors)
    {
        var displays = Display.GetAllDisplays();
        noMonitors = displays.Count == 0;

        displays = displays.FindAll(display => display.DisplayName
            .Equals(DISPLAY_ID, StringComparison.OrdinalIgnoreCase));

        noMonitors = displays.Count == 0 && noMonitors;
        return displays;
    }

    public static List<Display> GetDisplays()
    {
        return GetDisplays(out _);
    }

    public static Device.Status QueryStatus(out Version version)
    {
        return Device.QueryStatus(CLASS_GUID, HARDWARE_ID, out version);
    }

    public static bool GetVersion(IntPtr vdd, out string version)
    {
        if (IoControl(vdd, IoCtlCode.IOCTL_VERSION, null, out int vernum, 100))
        {
            int major = (vernum >> 16) & 0xFFFF;
            int minor = vernum & 0xFFFF;
            version = $"{major}.{minor}";
            return true;
        }

        version = "(unknown)";
        return false;
    }

    public static bool AddDisplay(IntPtr vdd, out int index)
    {
        if (IoControl(vdd, IoCtlCode.IOCTL_ADD, null, out index, 5000))
        {
            Update(vdd);
            return true;
        }

        return false;
    }

    public static bool RemoveDisplay(IntPtr vdd, int index)
    {
        var input = new byte[2];
        input[1] = (byte)(index & 0xFF);

        if (IoControl(vdd, IoCtlCode.IOCTL_REMOVE, input, 1000))
        {
            Update(vdd);
            return true;
        }

        return false;
    }

    public static bool Update(IntPtr vdd)
    {
        return IoControl(vdd, IoCtlCode.IOCTL_UPDATE, null, 1000);
    }

    private enum IoCtlCode
    {
        IOCTL_ADD = 0x22E004,
        IOCTL_REMOVE = 0x22A008,
        IOCTL_UPDATE = 0x22A00C,
        IOCTL_VERSION = 0x22E010,
        IOCTL_UNKNOWN1 = 0x22A014,
    }

    private static bool IoControl(IntPtr handle, IoCtlCode code, byte[]? input, int* result, int timeout)
    {
        var inBuffer = new byte[32];
        var overlapped = new Native.OVERLAPPED();

        if (input != null && input.Length > 0)
        {
            Array.Copy(input, inBuffer, Math.Min(input.Length, inBuffer.Length));
        }

        fixed (byte* buffer = inBuffer)
        {
            int outputLength = result != null ? sizeof(int) : 0;
            overlapped.hEvent = Native.CreateEvent(null, false, false, null);

            bool sent = Native.DeviceIoControl(handle, (uint)code,
                buffer, inBuffer.Length,
                result, outputLength,
                null, ref overlapped);

#if DEBUG
            if (code != IoCtlCode.IOCTL_UPDATE)
            {
                Debug.WriteLine(string.Format(
                    "[D] IoControl: {0}\n    Sent: {1}, error: {2}",
                    code,
                    sent,
                    DumpErrorCode(Marshal.GetLastWin32Error())));
            }
#endif
            if (!sent && Marshal.GetLastWin32Error() == 0x6)
            {
                if (overlapped.hEvent != IntPtr.Zero)
                {
                    Native.CloseHandle(overlapped.hEvent);
                }
                return false;
            }

            bool success = Native.GetOverlappedResultEx(handle, ref overlapped,
                out var numberOfBytesTransferred, timeout, false);

#if DEBUG
            if (code != IoCtlCode.IOCTL_UPDATE)
            {
                Debug.WriteLine(string.Format(
                    "    OverlappedResult: {0}, error: {1}",
                    success,
                    DumpErrorCode(Marshal.GetLastWin32Error())));
            }
#endif

            if (overlapped.hEvent != IntPtr.Zero)
            {
                Native.CloseHandle(overlapped.hEvent);
            }

            return success;
        }
    }

    private static bool IoControl(IntPtr handle, IoCtlCode code, byte[]? input, int timeout)
    {
        return IoControl(handle, code, input, null, timeout);
    }

    private static bool IoControl(IntPtr handle, IoCtlCode code, byte[]? input, out int result, int timeout)
    {
        int output;
        bool success = IoControl(handle, code, input, &output, timeout);
        result = output;
        return success;
    }

    private static string DumpErrorCode(int code)
    {
        string ret = code.ToString("X");

        if (code == 0)
        {
            ret += " (SUCCESS)";
        }
        else if (code == 0x6)
        {
            ret += " (ERROR_INVALID_HANDLE)";
        }
        else if (code == 0x3E5)
        {
            ret += " (ERROR_IO_PENDING)";
        }

        return ret;
    }

    private static class Native
    {
        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool DeviceIoControl(
            IntPtr device,
            uint code,
            void* lpInBuffer,
            int nInBufferSize,
            void* lpOutBuffer,
            int nOutBufferSize,
            void* lpBytesReturned,
            ref OVERLAPPED lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool GetOverlappedResultEx(
            IntPtr handle,
            ref OVERLAPPED lpOverlapped,
            out uint lpNumberOfBytesTransferred,
            int dwMilliseconds,
            [MarshalAs(UnmanagedType.Bool)] bool bAlertable);

        [StructLayout(LayoutKind.Sequential)]
        public struct OVERLAPPED
        {
            public IntPtr Internal;
            public IntPtr InternalHigh;
            public IntPtr Pointer;
            public IntPtr hEvent;
        }

        [DllImport("kernel32.dll", EntryPoint = "CreateEventW", CharSet = CharSet.Unicode)]
        public static extern IntPtr CreateEvent(
            void* lpEventAttributes,
            [MarshalAs(UnmanagedType.Bool)] bool bManualReset,
            [MarshalAs(UnmanagedType.Bool)] bool bInitialState,
            string? lpName);

        [DllImport("kernel32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr handle);
    }
}
