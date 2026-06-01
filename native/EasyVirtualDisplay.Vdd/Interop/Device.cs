using System;
using System.Runtime.InteropServices;
using System.Text;

namespace EasyVirtualDisplay.Vdd.Interop;

internal static unsafe class Device
{
    public enum Status
    {
        OK,
        INACCESSIBLE,
        UNKNOWN,
        UNKNOWN_PROBLEM,
        DISABLED,
        DRIVER_ERROR,
        RESTART_REQUIRED,
        DISABLED_SERVICE,
        NOT_INSTALLED
    }

    public static Status QueryStatus(string classGuid, string hardwareId, out Version driverVersion)
    {
        var status = Status.INACCESSIBLE;
        driverVersion = new Version(0, 0, 0, 0);

        var guid = Guid.Parse(classGuid);
        var devInfo = Native.SetupDiGetClassDevsA(ref guid, null, null, Native.DIGCF_PRESENT);

        if (devInfo.IsValidHandle())
        {
            status = Status.NOT_INSTALLED;

            const int hwndBufSize = 512;
            byte* hwidBuffer = stackalloc byte[hwndBufSize];

            var devInfoData = new Native.SP_DEVINFO_DATA();
            devInfoData.cbSize = sizeof(Native.SP_DEVINFO_DATA);

            for (uint index = 0; Native.SetupDiEnumDeviceInfo(devInfo, index, &devInfoData); index++)
            {
                uint regDataType = 0;

                if (Native.SetupDiGetDeviceRegistryPropertyA(devInfo, &devInfoData,
                    Native.SPDRP_HARDWAREID, &regDataType, hwidBuffer, hwndBufSize, null))
                {
                    if (regDataType == Native.REG_SZ || regDataType == Native.REG_MULTI_SZ)
                    {
                        var length = Native.lstrlenA(hwidBuffer);
                        if (length == hardwareId.Length)
                        {
                            var str = Marshal.PtrToStringAnsi((IntPtr)hwidBuffer, length);
                            if (string.Compare(str, hardwareId) == 0)
                            {
                                status = GetDeviceStatus(devInfoData.DevInst);
                                driverVersion = GetDeviceDriverVersion(devInfoData.DevInst);
                                break;
                            }
                        }
                    }
                }
            }

            Native.SetupDiDestroyDeviceInfoList(devInfo);
        }

        return status;
    }

    public static bool OpenHandle(string guid, out IntPtr handle)
    {
        handle = IntPtr.Zero;

        var interfaceGuid = Guid.Parse(guid);
        var devInfo = Native.SetupDiGetClassDevsA(ref interfaceGuid,
            null, null, Native.DIGCF_PRESENT | Native.DIGCF_DEVICEINTERFACE);

        if (devInfo != Native.INVALID_HANDLE_VALUE)
        {
            var devInterface = new Native.SP_DEVICE_INTERFACE_DATA();
            devInterface.cbSize = sizeof(Native.SP_DEVICE_INTERFACE_DATA);

            for (uint i = 0; Native.SetupDiEnumDeviceInterfaces(devInfo, null, ref interfaceGuid, i, &devInterface); ++i)
            {
                int detailSize = 0;
                Native.SetupDiGetDeviceInterfaceDetailA(devInfo, &devInterface, null, 0, &detailSize, null);

                var detail = (Native.SP_DEVICE_INTERFACE_DETAIL_DATA_A*)Marshal.AllocHGlobal(detailSize);
                detail->cbSize = sizeof(Native.SP_DEVICE_INTERFACE_DETAIL_DATA_A);

                if (Native.SetupDiGetDeviceInterfaceDetailA(devInfo, &devInterface, detail, detailSize, &detailSize, null))
                {
                    handle = Native.CreateFileA(&detail->DevicePath,
                        Native.GENERIC_READ | Native.GENERIC_WRITE,
                        Native.FILE_SHARE_READ | Native.FILE_SHARE_WRITE,
                        null,
                        Native.OPEN_EXISTING,
                        Native.FILE_ATTRIBUTE_NORMAL | Native.FILE_FLAG_NO_BUFFERING | Native.FILE_FLAG_OVERLAPPED | Native.FILE_FLAG_WRITE_THROUGH,
                        null);

                    if (handle.IsValidHandle())
                    {
                        Marshal.FreeHGlobal((IntPtr)detail);
                        break;
                    }
                }

                Marshal.FreeHGlobal((IntPtr)detail);
            }

            Native.SetupDiDestroyDeviceInfoList(devInfo);
        }

        return handle.IsValidHandle();
    }

    public static void CloseHandle(IntPtr handle)
    {
        if (handle.IsValidHandle())
        {
            Native.CloseHandle(handle);
        }
    }

    private static Status GetDeviceStatus(uint devInst)
    {
        uint devStatus;
        uint devProblemNum;

        if (Native.CM_Get_DevNode_Status(&devStatus, &devProblemNum, devInst, 0) != Native.CR_SUCCESS)
        {
            return Status.NOT_INSTALLED;
        }

        if ((devStatus & (Native.DN_DRIVER_LOADED | Native.DN_STARTED)) != 0)
        {
            return Status.OK;
        }

        if ((devStatus & Native.DN_HAS_PROBLEM) != 0)
        {
            switch (devProblemNum)
            {
                case Native.CM_PROB_NEED_RESTART:
                    return Status.RESTART_REQUIRED;

                case Native.CM_PROB_DISABLED:
                case Native.CM_PROB_HARDWARE_DISABLED:
                    return Status.DISABLED;

                case Native.CM_PROB_DISABLED_SERVICE:
                    return Status.DISABLED_SERVICE;

                default:
                    return devProblemNum == Native.CM_PROB_FAILED_POST_START
                        ? Status.DRIVER_ERROR : Status.UNKNOWN_PROBLEM;
            }
        }

        return Status.UNKNOWN;
    }

    public static string GetDeviceDescription(uint devInst)
    {
        uint propType;
        int length = 128 * sizeof(ushort);
        var buffer = stackalloc byte[length];

        uint cr = Native.CM_Get_DevNode_PropertyW(devInst,
            ref Native.DEVPROPKEY.Device_DeviceDesc, &propType, buffer, &length, 0);

        if (cr != Native.CR_SUCCESS)
        {
            return string.Empty;
        }

        return Marshal.PtrToStringUni((IntPtr)buffer) ?? string.Empty;
    }

    public static DateTime GetDeviceLastArrival(uint devInst)
    {
        uint propType;
        long lastArrival;
        int bufferLength = sizeof(long);

        uint cr = Native.CM_Get_DevNode_PropertyW(devInst,
            ref Native.DEVPROPKEY.Device_LastArrivalDate, &propType, &lastArrival, &bufferLength, 0);

        if (cr != Native.CR_SUCCESS)
        {
            return DateTime.MinValue;
        }

        return DateTime.FromFileTime(lastArrival);
    }

    public static bool GetParentDeviceInstance(uint devInst, out uint parentInst, out string parentId)
    {
        if (Native.CM_Get_Parent(out parentInst, devInst, 0) == 0)
        {
            byte[] idBuffer = new byte[Native.MAX_DEVICE_ID_LEN];
            Native.CM_Get_Device_IDA(parentInst, idBuffer, idBuffer.Length, 0);

            parentId = Encoding.ASCII.GetString(idBuffer).TrimEnd('\0');
            return true;
        }

        parentInst = 0;
        parentId = string.Empty;
        return false;
    }

    public static Version GetDeviceDriverVersion(uint devInst)
    {
        int length = 64;
        var buffer = stackalloc byte[length];

        uint propType;
        uint cr = Native.CM_Get_DevNode_PropertyW(devInst,
            ref Native.DEVPROPKEY.Device_DriverVersion, &propType, buffer, &length, 0);

        if (cr != Native.CR_SUCCESS)
        {
            return new Version(0, 0, 0, 0);
        }

        var vstr = Marshal.PtrToStringUni((IntPtr)buffer);
        return Version.TryParse(vstr, out var version) ? version : new Version(0, 0, 0, 0);
    }

    public static bool GetDeviceInstance(string deviceId, out uint devInst)
    {
        return Native.CM_Locate_DevNodeA(out devInst, deviceId, 0) == 0;
    }

    public static bool IsValidHandle(this IntPtr handle)
    {
        return handle != IntPtr.Zero
            && handle != (IntPtr)(-1);
    }

    private static class Native
    {
        public const int MAX_DEVICE_ID_LEN = 200;
        public static readonly IntPtr INVALID_HANDLE_VALUE = (IntPtr)(-1);

        public const uint GENERIC_READ = 0x80000000;
        public const uint GENERIC_WRITE = 0x40000000;

        public const uint FILE_SHARE_READ = 0x1;
        public const uint FILE_SHARE_WRITE = 0x2;

        public const uint OPEN_EXISTING = 3;

        public const uint FILE_ATTRIBUTE_NORMAL = 0x00000080;
        public const uint FILE_FLAG_NO_BUFFERING = 0x20000000;
        public const uint FILE_FLAG_OVERLAPPED = 0x40000000;
        public const uint FILE_FLAG_WRITE_THROUGH = 0x80000000;

        public const uint DIGCF_PRESENT = 0x2;
        public const uint DIGCF_DEVICEINTERFACE = 0x10;

        public const uint SPDRP_HARDWAREID = 0x1;

        public const uint REG_SZ = 1;
        public const uint REG_MULTI_SZ = 7;

        public const uint CR_SUCCESS = 0;

        public const uint CM_PROB_NEED_RESTART = 0x0000000E;
        public const uint CM_PROB_DISABLED = 0x00000016;
        public const uint CM_PROB_HARDWARE_DISABLED = 0x0000001D;
        public const uint CM_PROB_DISABLED_SERVICE = 0x00000020;
        public const uint CM_PROB_FAILED_POST_START = 0x0000002B;

        public const uint DN_DRIVER_LOADED = 0x00000002;
        public const uint DN_STARTED = 0x00000008;
        public const uint DN_HAS_PROBLEM = 0x00000400;

        [DllImport("kernel32.dll", CallingConvention = CallingConvention.Cdecl)]
        public static extern int lstrlenA(byte* lpString);

        [DllImport("kernel32.dll")]
        public static extern IntPtr CreateFileA(
            char* lpFileName,
            uint dwDesiredAccess,
            uint dwShareMode,
            void* lpSecurityAttributes,
            uint dwCreationDisposition,
            uint dwFlagsAndAttributes,
            void* hTemplateFile);

        [DllImport("kernel32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr handle);

        [StructLayout(LayoutKind.Sequential)]
        public struct OVERLAPPED
        {
            public IntPtr Internal;
            public IntPtr InternalHigh;
            public IntPtr Pointer;
            public IntPtr hEvent;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct SP_DEVICE_INTERFACE_DATA
        {
            public int cbSize;
            public Guid InterfaceClassGuid;
            public uint Flags;
            private IntPtr _reserved;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct SP_DEVINFO_DATA
        {
            public int cbSize;
            public Guid ClassGuid;
            public uint DevInst;
            private IntPtr _reserved;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct SP_DEVICE_INTERFACE_DETAIL_DATA_A
        {
            public int cbSize;
            public char DevicePath;
        }

        [DllImport("setupapi.dll", SetLastError = true)]
        public static extern IntPtr SetupDiGetClassDevsA(
            ref Guid classGuid,
            void* enumerator,
            void* hwndParent,
            uint flags);

        [DllImport("setupapi.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetupDiEnumDeviceInterfaces(
            IntPtr deviceInfoSet,
            SP_DEVINFO_DATA* deviceInfoData,
            ref Guid interfaceClassGuid,
            uint memberIndex,
            SP_DEVICE_INTERFACE_DATA* deviceInterfaceData);

        [DllImport("setupapi.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetupDiGetDeviceInterfaceDetailA(
            IntPtr deviceInfoSet,
            SP_DEVICE_INTERFACE_DATA* deviceInterfaceData,
            void* deviceInterfaceDetailData,
            int deviceInterfaceDetailDataSize,
            int* requiredSize,
            SP_DEVINFO_DATA* deviceInfoData);

        [DllImport("setupapi.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetupDiDestroyDeviceInfoList(
            IntPtr deviceInfoSet);

        [DllImport("setupapi.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetupDiEnumDeviceInfo(
            IntPtr deviceInfoSet,
            uint memberIndex,
            SP_DEVINFO_DATA* deviceInfoData);

        [DllImport("setupapi.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetupDiGetDeviceRegistryPropertyA(
            IntPtr deviceInfoSet,
            SP_DEVINFO_DATA* deviceInfoData,
            uint property,
            uint* propertyRegDataType,
            void* propertyBuffer,
            int propertyBufferSize,
            int* requiredSize);

        [DllImport("setupapi.dll")]
        public static extern uint CM_Get_DevNode_Status(
            uint* pulStatus,
            uint* pulProblemNumber,
            uint dnDevInst,
            uint ulFlags);

        [DllImport("setupapi.dll")]
        public static extern uint CM_Get_Parent(out uint pdnDevInst, uint dnDevInst, uint ulFlags);

        [DllImport("setupapi.dll")]
        public static extern uint CM_Get_Device_IDA(uint dnDevInst, byte[] buffer, int bufferLen, uint ulFlags);

        [StructLayout(LayoutKind.Sequential)]
        public struct DEVPROPKEY
        {
            public Guid fmtid;
            public uint pid;

            public static DEVPROPKEY Device_LastArrivalDate = new()
            {
                fmtid = Guid.Parse("{83DA6326-97A6-4088-9453-A1923F573B29}"),
                pid = 102,
            };

            public static DEVPROPKEY Device_DeviceDesc = new()
            {
                fmtid = Guid.Parse("{A45C254E-DF1C-4EFD-8020-67D146A850E0}"),
                pid = 2,
            };

            public static DEVPROPKEY Device_DriverVersion = new()
            {
                fmtid = Guid.Parse("{A8B865DD-2E3D-4094-AD97-E593A70C75D6}"),
                pid = 3,
            };
        }

        [DllImport("CfgMgr32.dll")]
        public static extern uint CM_Get_DevNode_PropertyW(
            uint dnDevInst,
            ref DEVPROPKEY propertyKey,
            uint* propertyType,
            void* propertyBuffer,
            int* propertyBufferSize,
            uint ulFlags);

        [DllImport("CfgMgr32.dll", CharSet = CharSet.Ansi)]
        public static extern uint CM_Locate_DevNodeA(out uint devInst, string deviceId, uint flags);
    }
}
