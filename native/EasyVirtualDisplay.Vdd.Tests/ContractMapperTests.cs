using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Interop;
using EasyVirtualDisplay.Vdd.Services;

namespace EasyVirtualDisplay.Vdd.Tests;

public class ContractMapperTests
{
    [Fact]
    public void MapStatus_MapsAllDeviceStatuses()
    {
        Assert.Equal(DriverStatuses.Ok, ContractMapper.MapStatus(Device.Status.OK));
        Assert.Equal(DriverStatuses.Inaccessible, ContractMapper.MapStatus(Device.Status.INACCESSIBLE));
        Assert.Equal(DriverStatuses.Unknown, ContractMapper.MapStatus(Device.Status.UNKNOWN));
        Assert.Equal(DriverStatuses.UnknownProblem, ContractMapper.MapStatus(Device.Status.UNKNOWN_PROBLEM));
        Assert.Equal(DriverStatuses.Disabled, ContractMapper.MapStatus(Device.Status.DISABLED));
        Assert.Equal(DriverStatuses.DriverError, ContractMapper.MapStatus(Device.Status.DRIVER_ERROR));
        Assert.Equal(DriverStatuses.RestartRequired, ContractMapper.MapStatus(Device.Status.RESTART_REQUIRED));
        Assert.Equal(DriverStatuses.DisabledService, ContractMapper.MapStatus(Device.Status.DISABLED_SERVICE));
        Assert.Equal(DriverStatuses.NotInstalled, ContractMapper.MapStatus(Device.Status.NOT_INSTALLED));
    }

    [Fact]
    public void MapOrientation_MapsAllValues()
    {
        Assert.Equal(Orientations.Landscape, ContractMapper.MapOrientation(Display.Orientation.Landscape));
        Assert.Equal(Orientations.Portrait, ContractMapper.MapOrientation(Display.Orientation.Portrait));
        Assert.Equal(Orientations.LandscapeFlipped, ContractMapper.MapOrientation(Display.Orientation.Landscape_Flipped));
        Assert.Equal(Orientations.PortraitFlipped, ContractMapper.MapOrientation(Display.Orientation.Portrait_Flipped));
    }

    [Fact]
    public void ParseOrientation_MapsAllValues()
    {
        Assert.Equal(Display.Orientation.Landscape, ContractMapper.ParseOrientation(Orientations.Landscape));
        Assert.Equal(Display.Orientation.Portrait, ContractMapper.ParseOrientation(Orientations.Portrait));
        Assert.Equal(Display.Orientation.Landscape_Flipped, ContractMapper.ParseOrientation(Orientations.LandscapeFlipped));
        Assert.Equal(Display.Orientation.Portrait_Flipped, ContractMapper.ParseOrientation(Orientations.PortraitFlipped));
    }

    [Fact]
    public void ParseOrientation_InvalidValue_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => ContractMapper.ParseOrientation("diagonal"));
    }

    [Fact]
    public void MapParentGpu_MapsAllValues()
    {
        Assert.Equal(ParentGpus.Auto, ContractMapper.MapParentGpu(Utils.ParentGPU.Auto));
        Assert.Equal(ParentGpus.Nvidia, ContractMapper.MapParentGpu(Utils.ParentGPU.NVIDIA));
        Assert.Equal(ParentGpus.Amd, ContractMapper.MapParentGpu(Utils.ParentGPU.AMD));
    }

    [Fact]
    public void ParseParentGpu_MapsAllValues()
    {
        Assert.Equal(Utils.ParentGPU.Auto, ContractMapper.ParseParentGpu(ParentGpus.Auto));
        Assert.Equal(Utils.ParentGPU.NVIDIA, ContractMapper.ParseParentGpu(ParentGpus.Nvidia));
        Assert.Equal(Utils.ParentGPU.AMD, ContractMapper.ParseParentGpu(ParentGpus.Amd));
    }

    [Fact]
    public void ParseParentGpu_InvalidValue_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => ContractMapper.ParseParentGpu("intel"));
    }

    [Fact]
    public void MapMode_MapsFieldsCorrectly()
    {
        var mode = new Display.Mode(1920, 1080, 60);
        var result = ContractMapper.MapMode(mode);
        Assert.Equal(1920, result.Width);
        Assert.Equal(1080, result.Height);
        Assert.Equal(60, result.Hz);
    }

    [Fact]
    public void CreateEmptySnapshot_HasExpectedDefaults()
    {
        var snapshot = ContractMapper.CreateEmptySnapshot();
        Assert.Equal(0, snapshot.Revision);
        Assert.Equal(DriverStatuses.Unknown, snapshot.Status);
        Assert.Empty(snapshot.Displays);
        Assert.Empty(snapshot.CustomModes);
        Assert.Equal(ParentGpus.Auto, snapshot.ParentGpu);
    }
}
