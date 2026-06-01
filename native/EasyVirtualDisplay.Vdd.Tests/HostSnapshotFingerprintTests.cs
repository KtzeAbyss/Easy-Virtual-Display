using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Services;

namespace EasyVirtualDisplay.Vdd.Tests;

public class HostSnapshotFingerprintTests
{
    private static HostSnapshot MakeSnapshot(long revision = 1, string status = DriverStatuses.Ok)
    {
        return new HostSnapshot
        {
            Revision = revision,
            Status = status,
            DriverVersion = "4.22.0.0",
            MaxDisplays = 4,
            Displays = Array.Empty<DisplaySummary>(),
            CustomModes = Array.Empty<DisplayMode>(),
            ParentGpu = ParentGpus.Auto,
        };
    }

    [Fact]
    public void SameContent_ProducesSameFingerprint()
    {
        var a = MakeSnapshot(revision: 1);
        var b = MakeSnapshot(revision: 1);
        Assert.Equal(HostSnapshotFingerprint.Compute(a), HostSnapshotFingerprint.Compute(b));
    }

    [Fact]
    public void DifferentRevision_ProducesSameFingerprint()
    {
        var a = MakeSnapshot(revision: 1);
        var b = MakeSnapshot(revision: 99);
        Assert.Equal(HostSnapshotFingerprint.Compute(a), HostSnapshotFingerprint.Compute(b));
    }

    [Fact]
    public void DifferentStatus_ProducesDifferentFingerprint()
    {
        var a = MakeSnapshot(status: DriverStatuses.Ok);
        var b = MakeSnapshot(status: DriverStatuses.NotInstalled);
        Assert.NotEqual(HostSnapshotFingerprint.Compute(a), HostSnapshotFingerprint.Compute(b));
    }

    [Fact]
    public void AddingDisplay_ProducesDifferentFingerprint()
    {
        var a = MakeSnapshot();
        var b = new HostSnapshot
        {
            Revision = a.Revision,
            Status = a.Status,
            DriverVersion = a.DriverVersion,
            MaxDisplays = a.MaxDisplays,
            Displays = new[]
            {
                new DisplaySummary
                {
                    Index = 0,
                    Identifier = 256,
                    DeviceName = "\\\\.\\DISPLAY1",
                    DisplayName = "Virtual Display 1",
                    Active = true,
                    CurrentMode = new DisplayMode { Width = 1920, Height = 1080, Hz = 60 },
                    CurrentOrientation = Orientations.Landscape,
                    SupportedResolutions = Array.Empty<SupportedResolution>(),
                    UnsupportedCurrentMode = false,
                },
            },
            CustomModes = a.CustomModes,
            ParentGpu = a.ParentGpu,
        };

        Assert.NotEqual(HostSnapshotFingerprint.Compute(a), HostSnapshotFingerprint.Compute(b));
    }

    [Fact]
    public void DifferentCustomModes_ProducesDifferentFingerprint()
    {
        var a = MakeSnapshot();
        var b = new HostSnapshot
        {
            Revision = a.Revision,
            Status = a.Status,
            DriverVersion = a.DriverVersion,
            MaxDisplays = a.MaxDisplays,
            Displays = a.Displays,
            CustomModes = new[] { new DisplayMode { Width = 2560, Height = 1440, Hz = 144 } },
            ParentGpu = a.ParentGpu,
        };

        Assert.NotEqual(HostSnapshotFingerprint.Compute(a), HostSnapshotFingerprint.Compute(b));
    }
}
