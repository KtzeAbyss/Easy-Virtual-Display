using System.Collections.Generic;
using System.Text.Json;
using EasyVirtualDisplay.Host.Hosting;
using EasyVirtualDisplay.Vdd.Domain;

namespace EasyVirtualDisplay.Host.Errors;

public static class HostErrorMapper
{
    internal static HostError Normalize(Exception exception)
    {
        ArgumentNullException.ThrowIfNull(exception);

        return exception switch
        {
            ErrorDriverStatus ex => FromDriverStatus(ex.Status),
            ErrorExceededLimit ex => new HostError
            {
                Code = "limit_exceeded",
                Message = ex.Message,
                Details = new Dictionary<string, object?>
                {
                    ["limit"] = ex.Limit,
                },
            },
            ErrorDisplayNotFound ex => new HostError
            {
                Code = "display_not_found",
                Message = ex.Message,
                Details = new Dictionary<string, object?>
                {
                    ["index"] = ex.Index,
                },
            },
            ErrorUnsupportedMode ex => new HostError
            {
                Code = "unsupported_mode",
                Message = ex.Message,
                Details = new Dictionary<string, object?>
                {
                    ["index"] = ex.Index,
                    ["width"] = ex.Width,
                    ["height"] = ex.Height,
                    ["hz"] = ex.Hz,
                    ["orientation"] = ex.Orientation,
                },
            },
            ErrorDeviceHandle => new HostError
            {
                Code = "driver_error",
                Message = "Failed to obtain the driver device handle.",
            },
            ErrorOperationFailed ex => new HostError
            {
                Code = ex.Code ?? "driver_error",
                Message = ex.Message,
                Details = new Dictionary<string, object?>
                {
                    ["operation"] = ex.Type.ToString(),
                },
            },
            ArgumentException ex => new HostError
            {
                Code = "driver_error",
                Message = ex.Message,
            },
            JsonException ex => new HostError
            {
                Code = "driver_error",
                Message = $"Invalid JSON payload: {ex.Message}",
            },
            _ => new HostError
            {
                Code = "driver_error",
                Message = string.IsNullOrWhiteSpace(exception.Message)
                    ? "Native host operation failed."
                    : exception.Message,
            },
        };
    }

    public static string ToMessage(Exception exception)
    {
        return JsonSerializer.Serialize(Normalize(exception), HostJsonOptions.Default);
    }

    private static HostError FromDriverStatus(DriverState status)
    {
        return status switch
        {
            DriverState.NotInstalled => new HostError
            {
                Code = "driver_not_installed",
                Message = "The virtual display driver is not installed.",
            },
            DriverState.Disabled => new HostError
            {
                Code = "driver_disabled",
                Message = "The virtual display driver is disabled.",
            },
            DriverState.DisabledService => new HostError
            {
                Code = "driver_disabled",
                Message = "The virtual display driver service is disabled.",
            },
            DriverState.RestartRequired => new HostError
            {
                Code = "driver_restart_required",
                Message = "The virtual display driver needs a restart.",
            },
            DriverState.Inaccessible => new HostError
            {
                Code = "driver_error",
                Message = "The virtual display driver is inaccessible.",
            },
            DriverState.DriverError => new HostError
            {
                Code = "driver_error",
                Message = "The virtual display driver reported an error.",
            },
            DriverState.UnknownProblem => new HostError
            {
                Code = "driver_error",
                Message = "The virtual display driver reported an unknown problem.",
            },
            _ => new HostError
            {
                Code = "driver_error",
                Message = $"The virtual display driver is not ready (status: {status}).",
            },
        };
    }
}
