using System;
using System.Linq;
using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Services;

namespace EasyVirtualDisplay.Vdd.Tests;

public class VddConfigurationServiceTests
{
    [Fact]
    public void NormalizeModes_NullInput_Throws()
    {
        Assert.Throws<ArgumentNullException>(() =>
            VddConfigurationService.NormalizeModes(null!));
    }

    [Fact]
    public void NormalizeModes_ExceedsMax_Throws()
    {
        var modes = Enumerable.Range(0, VddConfigurationService.MaxCustomModes + 1)
            .Select(_ => new DisplayMode { Width = 1920, Height = 1080, Hz = 60 })
            .ToArray();

        var ex = Assert.Throws<ArgumentOutOfRangeException>(() =>
            VddConfigurationService.NormalizeModes(modes));
        Assert.Contains("No more than", ex.Message);
    }

    [Fact]
    public void NormalizeModes_ZeroWidth_Throws()
    {
        var modes = new[] { new DisplayMode { Width = 0, Height = 1080, Hz = 60 } };

        var ex = Assert.Throws<ArgumentOutOfRangeException>(() =>
            VddConfigurationService.NormalizeModes(modes));
        Assert.Contains("positive", ex.Message);
    }

    [Fact]
    public void NormalizeModes_ZeroHeight_Throws()
    {
        var modes = new[] { new DisplayMode { Width = 1920, Height = 0, Hz = 60 } };

        Assert.Throws<ArgumentOutOfRangeException>(() =>
            VddConfigurationService.NormalizeModes(modes));
    }

    [Fact]
    public void NormalizeModes_ZeroHz_Throws()
    {
        var modes = new[] { new DisplayMode { Width = 1920, Height = 1080, Hz = 0 } };

        Assert.Throws<ArgumentOutOfRangeException>(() =>
            VddConfigurationService.NormalizeModes(modes));
    }

    [Fact]
    public void NormalizeModes_NegativeValues_Throws()
    {
        var modes = new[] { new DisplayMode { Width = -1, Height = 1080, Hz = 60 } };

        Assert.Throws<ArgumentOutOfRangeException>(() =>
            VddConfigurationService.NormalizeModes(modes));
    }

    [Fact]
    public void NormalizeModes_ValidModes_ReturnsMappedList()
    {
        var modes = new[]
        {
            new DisplayMode { Width = 1920, Height = 1080, Hz = 60 },
            new DisplayMode { Width = 2560, Height = 1440, Hz = 144 },
        };

        var result = VddConfigurationService.NormalizeModes(modes);

        Assert.Equal(2, result.Count);
        Assert.Equal(1920, result[0].Width);
        Assert.Equal(1080, result[0].Height);
        Assert.Equal(60, result[0].Hz);
        Assert.Equal(2560, result[1].Width);
        Assert.Equal(1440, result[1].Height);
        Assert.Equal(144, result[1].Hz);
    }

    [Fact]
    public void NormalizeModes_EmptyList_ReturnsEmptyList()
    {
        var result = VddConfigurationService.NormalizeModes(
            Array.Empty<DisplayMode>());

        Assert.Empty(result);
    }

    [Fact]
    public void MaxCustomModes_IsFive()
    {
        Assert.Equal(5, VddConfigurationService.MaxCustomModes);
    }
}
