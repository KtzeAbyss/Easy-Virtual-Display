using System.Collections.Generic;

namespace EasyVirtualDisplay.Host.Errors;

internal sealed class HostError
{
    public string Code { get; init; } = "driver_error";

    public string Message { get; init; } = "Native host operation failed.";

    public IReadOnlyDictionary<string, object?>? Details { get; init; }
}
