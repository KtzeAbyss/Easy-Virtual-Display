using System;
namespace EasyVirtualDisplay.Vdd.Domain;

public class ErrorDriverStatus : Exception
{
    public readonly DriverState Status;

    public ErrorDriverStatus(DriverState status)
        : base($"Driver status is '{status}'.")
    {
        Status = status;
    }
}

public class ErrorDeviceHandle : Exception
{
    public ErrorDeviceHandle()
        : base("Failed to obtain the driver device handle.")
    {
    }
}

public class ErrorExceededLimit : Exception
{
    public readonly int Limit;

    public ErrorExceededLimit(int limit)
        : base($"Exceeded limit ({limit}), could not add more displays.")
    {
        Limit = limit;
    }
}

public class ErrorOperationFailed : Exception
{
    public enum Operation
    {
        AddDisplay,
        RemoveDisplay,
        Uninstall,
    }

    public readonly Operation Type;

    public readonly string? Code;

    public ErrorOperationFailed(Operation type)
        : base(type == Operation.AddDisplay
            ? "Failed to add a virtual display."
            : type == Operation.RemoveDisplay
            ? "Failed to remove the virtual display."
            : "Driver operation failed.")
    {
        Type = type;
    }

    public ErrorOperationFailed(Operation type, string code, string message)
        : base(message)
    {
        Type = type;
        Code = code;
    }
}

public class ErrorDisplayNotFound : Exception
{
    public readonly int Index;

    public ErrorDisplayNotFound(int index)
        : base($"Display index {index} is not found.")
    {
        Index = index;
    }
}

public class ErrorUnsupportedMode : Exception
{
    public readonly int Index;

    public readonly int? Width;

    public readonly int? Height;

    public readonly int? Hz;

    public readonly string? Orientation;

    public ErrorUnsupportedMode(int index, int? width, int? height, int? hz, string? orientation)
        : base($"Display index {index} does not support the requested mode.")
    {
        Index = index;
        Width = width;
        Height = height;
        Hz = hz;
        Orientation = orientation;
    }
}
