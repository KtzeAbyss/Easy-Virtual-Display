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

    public static List<Display> OwnDisplays(List<Display> allDisplays)
    {
        return allDisplays.FindAll(display => display.DisplayName
            .Equals(DISPLAY_ID, StringComparison.OrdinalIgnoreCase));
    }

    public static List<Display> GetDisplays()
    {
        return OwnDisplays(Display.GetAllDisplays());
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

        if (IoControl(vdd, IoCtlCode.IOCTL_REMOVE, input, 5000))
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

    private const int InputLength = 32;
    private const int ERROR_IO_PENDING = 0x3E5;
    private const int WAIT_TIMEOUT = 0x102;

    // How long a timed-out request may take to wind down after CancelIoEx.
    private const int CancelGraceMilliseconds = 1000;

    /// <summary>
    /// Everything the driver may write to until the request completes. It lives in native
    /// memory because completion can come after the caller has timed out and returned.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct Request
    {
        public Native.OVERLAPPED Overlapped;
        public int Output;
        public fixed byte Input[InputLength];
    }

    private static bool IoControl(IntPtr handle, IoCtlCode code, byte[]? input, int* result, int timeout)
    {
        var request = (Request*)NativeMemory.AllocZeroed((nuint)sizeof(Request));

        if (input != null && input.Length > 0)
        {
            input.AsSpan(0, Math.Min(input.Length, InputLength)).CopyTo(new Span<byte>(request->Input, InputLength));
        }

        request->Overlapped.hEvent = Native.CreateEvent(null, true, false, null);

        bool sent = Native.DeviceIoControl(handle, (uint)code,
            request->Input, InputLength,
            result != null ? &request->Output : null, result != null ? sizeof(int) : 0,
            null, &request->Overlapped);

        if (!sent && Marshal.GetLastWin32Error() != ERROR_IO_PENDING)
        {
            FreeRequest(request);
            return false;
        }

        bool success = Native.GetOverlappedResultEx(handle, &request->Overlapped,
            out _, timeout, false);

        if (!success && Marshal.GetLastWin32Error() == WAIT_TIMEOUT)
        {
            // Still in flight: cancel it and wait for the driver to let go of the request.
            Native.CancelIoEx(handle, &request->Overlapped);
            success = Native.GetOverlappedResultEx(handle, &request->Overlapped,
                out _, CancelGraceMilliseconds, false);

            if (!success && Marshal.GetLastWin32Error() == WAIT_TIMEOUT)
            {
                // The driver may still write to the request; leak it rather than free it.
#if DEBUG
                Debug.WriteLine($"[D] IoControl: {code} did not finish after cancel; abandoning it");
#endif
                return false;
            }
        }

#if DEBUG
        if (code != IoCtlCode.IOCTL_UPDATE)
        {
            Debug.WriteLine($"[D] IoControl: {code} success={success}");
        }
#endif

        if (success && result != null)
        {
            *result = request->Output;
        }

        FreeRequest(request);
        return success;
    }

    private static void FreeRequest(Request* request)
    {
        if (request->Overlapped.hEvent != IntPtr.Zero)
        {
            Native.CloseHandle(request->Overlapped.hEvent);
        }

        NativeMemory.Free(request);
    }

    private static bool IoControl(IntPtr handle, IoCtlCode code, byte[]? input, int timeout)
    {
        return IoControl(handle, code, input, null, timeout);
    }

    private static bool IoControl(IntPtr handle, IoCtlCode code, byte[]? input, out int result, int timeout)
    {
        int output = 0;
        bool success = IoControl(handle, code, input, &output, timeout);
        result = output;
        return success;
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
            OVERLAPPED* lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool GetOverlappedResultEx(
            IntPtr handle,
            OVERLAPPED* lpOverlapped,
            out uint lpNumberOfBytesTransferred,
            int dwMilliseconds,
            [MarshalAs(UnmanagedType.Bool)] bool bAlertable);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CancelIoEx(IntPtr handle, OVERLAPPED* lpOverlapped);

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
