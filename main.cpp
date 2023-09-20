#include <chrono>
#include <thread>
#include <tchar.h>
#include <strsafe.h>
#include <windows.h>
#include <SetupAPI.h>
#include <string>   
#include <iostream> 

#pragma comment(lib, "Setupapi.lib")

BOOLEAN GetDevicePath2(
    _In_ LPCGUID InterfaceGuid,
    _Out_writes_(BufLen) PTCHAR DevicePath,
    _In_ size_t BufLen
)
{
    HANDLE                              hDevice = INVALID_HANDLE_VALUE;
    PSP_DEVICE_INTERFACE_DETAIL_DATA    deviceInterfaceDetailData = NULL;
    ULONG                               predictedLength = 0;
    ULONG                               requiredLength = 0;
    HDEVINFO                            hardwareDeviceInfo;
    SP_DEVICE_INTERFACE_DATA deviceInterfaceData = { 0 };
    BOOLEAN                             status = FALSE;
    HRESULT                             hr;

    hardwareDeviceInfo = SetupDiGetClassDevs(
        InterfaceGuid,
        NULL, // Define no enumerator (global)
        NULL, // Define no
        (DIGCF_PRESENT | // Only Devices present
            DIGCF_DEVICEINTERFACE)); // Function class devices.
    if (INVALID_HANDLE_VALUE == hardwareDeviceInfo)
    {
        printf("Idd device: SetupDiGetClassDevs failed, last error 0x%x\n", GetLastError());
        return FALSE;
    }

    deviceInterfaceData.cbSize = sizeof(SP_DEVICE_INTERFACE_DATA);


    if (!SetupDiEnumDeviceInterfaces(hardwareDeviceInfo,
        0, // No care about specific PDOs
        InterfaceGuid,
        0, //
        &deviceInterfaceData))
    {
        printf("Idd device: SetupDiEnumDeviceInterfaces failed, last error 0x%x\n", GetLastError());
        goto Clean0;
    }

    //
    // Allocate a function class device data structure to receive the
    // information about this particular device.
    //
    SetupDiGetDeviceInterfaceDetail(
        hardwareDeviceInfo,
        &deviceInterfaceData,
        NULL, // probing so no output buffer yet
        0, // probing so output buffer length of zero
        &requiredLength,
        NULL);//not interested in the specific dev-node

    if (ERROR_INSUFFICIENT_BUFFER != GetLastError())
    {
        printf("Idd device: SetupDiGetDeviceInterfaceDetail failed, last error 0x%x\n", GetLastError());
        goto Clean0;
    }

    predictedLength = requiredLength;
    deviceInterfaceDetailData = (PSP_DEVICE_INTERFACE_DETAIL_DATA)HeapAlloc(
        GetProcessHeap(),
        HEAP_ZERO_MEMORY,
        predictedLength
    );

    if (deviceInterfaceDetailData)
    {
        deviceInterfaceDetailData->cbSize =
            sizeof(SP_DEVICE_INTERFACE_DETAIL_DATA);
    }
    else
    {
        printf("Idd device: HeapAlloc failed, last error 0x%x\n", GetLastError());
        goto Clean0;
    }

    if (!SetupDiGetDeviceInterfaceDetail(
        hardwareDeviceInfo,
        &deviceInterfaceData,
        deviceInterfaceDetailData,
        predictedLength,
        &requiredLength,
        NULL))
    {
        printf("Idd device: SetupDiGetDeviceInterfaceDetail failed, last error 0x%x\n", GetLastError());
        goto Clean1;
    }

    hr = StringCchCopy(DevicePath, BufLen, deviceInterfaceDetailData->DevicePath);
    if (FAILED(hr))
    {
        printf("Error: StringCchCopy failed with HRESULT 0x%x", hr);
        status = FALSE;
        goto Clean1;
    }
    else
    {
        status = TRUE;
    }

Clean1:
    (VOID)HeapFree(GetProcessHeap(), 0, deviceInterfaceDetailData);
Clean0:
    (VOID)SetupDiDestroyDeviceInfoList(hardwareDeviceInfo);
    return status;
}

HANDLE DeviceOpenHandle(const GUID& devGuid)
{
    // const int maxDevPathLen = 256;
    TCHAR devicePath[256] = { 0 };
    HANDLE hDevice = INVALID_HANDLE_VALUE;
    do
    {
        if (FALSE == GetDevicePath2(
            &devGuid,
            devicePath,
            sizeof(devicePath) / sizeof(devicePath[0])))
        {
            break;
        }
        if (_tcslen(devicePath) == 0)
        {
            printf("GetDevicePath got empty device path\n");
            break;
        }

        _tprintf(_T("Idd device: try open %s\n"), devicePath);
        hDevice = CreateFile(
            devicePath,
            GENERIC_READ | GENERIC_WRITE,
            // FILE_SHARE_READ | FILE_SHARE_WRITE,
            0,
            NULL, // no SECURITY_ATTRIBUTES structure
            OPEN_EXISTING, // No special create flags
            0, // No special attributes
            NULL
        );
        if (hDevice == INVALID_HANDLE_VALUE || hDevice == NULL)
        {
            DWORD error = GetLastError();
            printf("CreateFile failed 0x%lx\n", error);
        }
    } while (0);

    return hDevice;
}

enum VddCtlCode
{
    IOCTL_VDD_CONNECT = 0x22A008,
    IOCTL_VDD_ADD = 0x22E004,
    IOCTL_VDD_UPDATE = 0x22A00C,
};

void VddIoCtl(HANDLE vdd, VddCtlCode code)
{
    BYTE InBuffer[32]{};
    int OutBuffer = 0;
    OVERLAPPED Overlapped{};
    DWORD NumberOfBytesTransferred;

    Overlapped.hEvent = CreateEventW(NULL, NULL, NULL, NULL);
    DeviceIoControl(vdd, code, InBuffer, _countof(InBuffer), &OutBuffer, sizeof(OutBuffer), NULL, &Overlapped);
    GetOverlappedResult(vdd, &Overlapped, &NumberOfBytesTransferred, TRUE);

    if (Overlapped.hEvent && Overlapped.hEvent != INVALID_HANDLE_VALUE)
        CloseHandle(Overlapped.hEvent);
}

bool quit = false;

void pipe_thread()
{
    HANDLE hPipe;
    char buffer[1024] = { 0 };
    DWORD dwRead;

    hPipe = CreateNamedPipe(TEXT("\\\\.\\pipe\\MyPipe"),
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
        1,
        1024 * 16,
        1024 * 16,
        NMPWAIT_USE_DEFAULT_WAIT,
        NULL);

    if (hPipe != INVALID_HANDLE_VALUE)
    {
        if (ConnectNamedPipe(hPipe, NULL) != FALSE)   // wait for someone to connect to the pipe
        {
            while (ReadFile(hPipe, buffer, sizeof(buffer) - 1, &dwRead, NULL) != FALSE)
            {
                buffer[dwRead] = '\0';

                if (std::string(buffer) == "quit")
                {
                    quit = true;
                    break;
                }
            }
        }

        DisconnectNamedPipe(hPipe);
    }

    CloseHandle(hPipe);
}

int main()
{
    const GUID PARSEC_VDD_DEVINTERFACE = \
    { 0x00b41627, 0x04c4, 0x429e, { 0xa2, 0x6e, 0x02, 0x65, 0xcf, 0x50, 0xc8, 0xfa } };

    // try to get device handle with GUID
    HANDLE vdd = DeviceOpenHandle(PARSEC_VDD_DEVINTERFACE);
    if (!vdd || vdd == INVALID_HANDLE_VALUE)
    {
        printf("failed to get ParsecVDD device handle.\n");
        return 1;
    }

    // connect & plug in
    VddIoCtl(vdd, IOCTL_VDD_CONNECT);
    VddIoCtl(vdd, IOCTL_VDD_UPDATE);
    VddIoCtl(vdd, IOCTL_VDD_ADD);
    VddIoCtl(vdd, IOCTL_VDD_UPDATE);

    std::thread pipe_thread_instance(pipe_thread);

    while (true)
    {
        // update each 100ms
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        VddIoCtl(vdd, IOCTL_VDD_UPDATE);

        if (quit) {
            break;
        }
    }

    pipe_thread_instance.join();

    // disconnect
    VddIoCtl(vdd, IOCTL_VDD_CONNECT);
    CloseHandle(vdd);

    return 0;
}