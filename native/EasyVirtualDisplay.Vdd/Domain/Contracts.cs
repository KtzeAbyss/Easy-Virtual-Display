using System;
using System.Collections.Generic;

namespace EasyVirtualDisplay.Vdd.Domain;

public static class DriverStatuses
{
    public const string Ok = "ok";
    public const string Inaccessible = "inaccessible";
    public const string Unknown = "unknown";
    public const string UnknownProblem = "unknown_problem";
    public const string Disabled = "disabled";
    public const string DriverError = "driver_error";
    public const string RestartRequired = "restart_required";
    public const string DisabledService = "disabled_service";
    public const string NotInstalled = "not_installed";
}

public enum DriverState
{
    Ok,
    Inaccessible,
    Unknown,
    UnknownProblem,
    Disabled,
    DriverError,
    RestartRequired,
    DisabledService,
    NotInstalled,
}

public static class ParentGpus
{
    public const string Auto = "auto";
    public const string Nvidia = "nvidia";
    public const string Amd = "amd";
}

public static class Orientations
{
    public const string Landscape = "landscape";
    public const string Portrait = "portrait";
    public const string LandscapeFlipped = "landscape_flipped";
    public const string PortraitFlipped = "portrait_flipped";
}

public sealed class DisplayMode
{
    public int Width { get; init; }

    public int Height { get; init; }

    public int Hz { get; init; }
}

public sealed class SupportedResolution
{
    public int Width { get; init; }

    public int Height { get; init; }

    public IReadOnlyList<int> RefreshRates { get; init; } = Array.Empty<int>();
}

public sealed class DisplaySummary
{
    public int Index { get; init; }

    public int Identifier { get; init; }

    public string DeviceName { get; init; } = string.Empty;

    public string DisplayName { get; init; } = string.Empty;

    public bool Active { get; init; }

    public DisplayMode? CurrentMode { get; init; }

    public string CurrentOrientation { get; init; } = Orientations.Landscape;

    public IReadOnlyList<SupportedResolution> SupportedResolutions { get; init; } = Array.Empty<SupportedResolution>();

    public bool UnsupportedCurrentMode { get; init; }
}

public sealed class HostSnapshot
{
    public long Revision { get; init; }

    public string Status { get; init; } = DriverStatuses.Unknown;

    public string DriverVersion { get; init; } = "0.0.0.0";

    public int MaxDisplays { get; init; }

    public IReadOnlyList<DisplaySummary> Displays { get; init; } = Array.Empty<DisplaySummary>();

    public IReadOnlyList<DisplayMode> CustomModes { get; init; } = Array.Empty<DisplayMode>();

    public string ParentGpu { get; init; } = ParentGpus.Auto;
}

public sealed class SetDisplayModeInput
{
    public int Index { get; init; }

    public int? Width { get; init; }

    public int? Height { get; init; }

    public int? Hz { get; init; }

    public string? Orientation { get; init; }
}

public sealed class ApplyAdminConfigInput
{
    public IReadOnlyList<DisplayMode> CustomModes { get; init; } = Array.Empty<DisplayMode>();

    public string ParentGpu { get; init; } = ParentGpus.Auto;
}

public sealed class AdminConfigSnapshot
{
    public IReadOnlyList<DisplayMode> CustomModes { get; init; } = Array.Empty<DisplayMode>();

    public string ParentGpu { get; init; } = ParentGpus.Auto;
}
