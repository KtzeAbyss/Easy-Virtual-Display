using System;
using System.Collections.Generic;
using System.Linq;
using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Interop;

namespace EasyVirtualDisplay.Vdd.Services;

internal static class ContractMapper
{
    public static HostSnapshot CreateEmptySnapshot()
    {
        return new HostSnapshot
        {
            Revision = 0,
            Status = DriverStatuses.Unknown,
            DriverVersion = "0.0.0.0",
            MaxDisplays = Core.MAX_DISPLAYS,
            Displays = Array.Empty<DisplaySummary>(),
            CustomModes = Array.Empty<DisplayMode>(),
            ParentGpu = ParentGpus.Auto,
        };
    }

    public static HostSnapshot BuildSnapshot(
        long revision,
        Device.Status status,
        Version driverVersion,
        IReadOnlyList<Display> displays,
        AdminConfigSnapshot config)
    {
        return new HostSnapshot
        {
            Revision = revision,
            Status = MapStatus(status),
            DriverVersion = driverVersion.ToString(),
            MaxDisplays = Core.MAX_DISPLAYS,
            Displays = displays.Select(MapDisplay).ToArray(),
            CustomModes = config.CustomModes.ToArray(),
            ParentGpu = config.ParentGpu,
        };
    }

    public static string MapStatus(Device.Status status)
    {
        return MapStatus(ToDriverState(status));
    }

    public static string MapStatus(DriverState status)
    {
        return status switch
        {
            DriverState.Ok => DriverStatuses.Ok,
            DriverState.Inaccessible => DriverStatuses.Inaccessible,
            DriverState.Unknown => DriverStatuses.Unknown,
            DriverState.UnknownProblem => DriverStatuses.UnknownProblem,
            DriverState.Disabled => DriverStatuses.Disabled,
            DriverState.DriverError => DriverStatuses.DriverError,
            DriverState.RestartRequired => DriverStatuses.RestartRequired,
            DriverState.DisabledService => DriverStatuses.DisabledService,
            DriverState.NotInstalled => DriverStatuses.NotInstalled,
            _ => DriverStatuses.Unknown,
        };
    }

    public static DriverState ToDriverState(Device.Status status)
    {
        return status switch
        {
            Device.Status.OK => DriverState.Ok,
            Device.Status.INACCESSIBLE => DriverState.Inaccessible,
            Device.Status.UNKNOWN => DriverState.Unknown,
            Device.Status.UNKNOWN_PROBLEM => DriverState.UnknownProblem,
            Device.Status.DISABLED => DriverState.Disabled,
            Device.Status.DRIVER_ERROR => DriverState.DriverError,
            Device.Status.RESTART_REQUIRED => DriverState.RestartRequired,
            Device.Status.DISABLED_SERVICE => DriverState.DisabledService,
            Device.Status.NOT_INSTALLED => DriverState.NotInstalled,
            _ => DriverState.Unknown,
        };
    }

    public static string MapParentGpu(Utils.ParentGPU parentGpu)
    {
        return parentGpu switch
        {
            Utils.ParentGPU.Auto => ParentGpus.Auto,
            Utils.ParentGPU.NVIDIA => ParentGpus.Nvidia,
            Utils.ParentGPU.AMD => ParentGpus.Amd,
            _ => ParentGpus.Auto,
        };
    }

    public static Utils.ParentGPU ParseParentGpu(string parentGpu)
    {
        return parentGpu.Trim().ToLowerInvariant() switch
        {
            ParentGpus.Auto => Utils.ParentGPU.Auto,
            ParentGpus.Nvidia => Utils.ParentGPU.NVIDIA,
            ParentGpus.Amd => Utils.ParentGPU.AMD,
            _ => throw new ArgumentOutOfRangeException(nameof(parentGpu), parentGpu, "Unsupported parent GPU."),
        };
    }

    public static string MapOrientation(Display.Orientation orientation)
    {
        return orientation switch
        {
            Display.Orientation.Landscape => Orientations.Landscape,
            Display.Orientation.Portrait => Orientations.Portrait,
            Display.Orientation.Landscape_Flipped => Orientations.LandscapeFlipped,
            Display.Orientation.Portrait_Flipped => Orientations.PortraitFlipped,
            _ => Orientations.Landscape,
        };
    }

    public static Display.Orientation ParseOrientation(string orientation)
    {
        return orientation.Trim().ToLowerInvariant() switch
        {
            Orientations.Landscape => Display.Orientation.Landscape,
            Orientations.Portrait => Display.Orientation.Portrait,
            Orientations.LandscapeFlipped => Display.Orientation.Landscape_Flipped,
            Orientations.PortraitFlipped => Display.Orientation.Portrait_Flipped,
            _ => throw new ArgumentOutOfRangeException(nameof(orientation), orientation, "Unsupported orientation."),
        };
    }

    public static DisplayMode MapMode(Display.Mode mode)
    {
        return new DisplayMode
        {
            Width = mode.Width,
            Height = mode.Height,
            Hz = mode.Hz,
        };
    }

    private static DisplaySummary MapDisplay(Display display)
    {
        return new DisplaySummary
        {
            Index = display.DisplayIndex,
            Identifier = display.Identifier,
            DeviceName = display.DeviceName,
            DisplayName = display.DisplayName,
            Active = display.Active,
            CurrentMode = display.CurrentMode is null ? null : MapMode(display.CurrentMode),
            CurrentOrientation = MapOrientation(display.CurrentOrientation),
            SupportedResolutions = display.SupportedResolutions
                .Select(modeSet => new SupportedResolution
                {
                    Width = modeSet.Width,
                    Height = modeSet.Height,
                    RefreshRates = modeSet.RefreshRates.ToArray(),
                })
                .ToArray(),
            UnsupportedCurrentMode = IsUnsupportedCurrentMode(display),
        };
    }

    private static bool IsUnsupportedCurrentMode(Display display)
    {
        if (display.CurrentMode is null || display.SupportedResolutions.Count == 0)
        {
            return false;
        }

        return !display.SupportedResolutions.Any(resolution =>
            resolution.Width == display.CurrentMode.Width
            && resolution.Height == display.CurrentMode.Height
            && resolution.RefreshRates.Contains(display.CurrentMode.Hz));
    }
}
