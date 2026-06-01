using System;
using System.Collections.Generic;
using System.Linq;
using EasyVirtualDisplay.Vdd.Domain;

namespace EasyVirtualDisplay.Vdd.Services;

public sealed class VddConfigurationService
{
    public const int MaxCustomModes = 5;

    public AdminConfigSnapshot GetSnapshot()
    {
        return new AdminConfigSnapshot
        {
            CustomModes = Utils.GetCustomDisplayModes()
                .Select(ContractMapper.MapMode)
                .ToArray(),
            ParentGpu = ContractMapper.MapParentGpu(Utils.GetParentGPU()),
        };
    }

    public void Apply(ApplyAdminConfigInput input)
    {
        ArgumentNullException.ThrowIfNull(input);

        var modes = NormalizeModes(input.CustomModes);
        var parentGpu = ContractMapper.ParseParentGpu(input.ParentGpu);

        var previousModes = Utils.GetCustomDisplayModes();
        Utils.SetCustomDisplayModes(modes);

        try
        {
            Utils.SetParentGPU(parentGpu);
        }
        catch
        {
            try { Utils.SetCustomDisplayModes(previousModes.ToList()); } catch { }
            throw;
        }
    }

    internal static List<Display.Mode> NormalizeModes(IReadOnlyList<DisplayMode> modes)
    {
        ArgumentNullException.ThrowIfNull(modes);

        if (modes.Count > MaxCustomModes)
        {
            throw new ArgumentOutOfRangeException(
                nameof(modes),
                modes.Count,
                $"No more than {MaxCustomModes} custom modes are supported.");
        }

        return modes.Select(mode =>
            {
                if (mode.Width <= 0 || mode.Height <= 0 || mode.Hz <= 0)
                {
                    throw new ArgumentOutOfRangeException(nameof(modes), "Custom modes must use positive width, height, and hz values.");
                }

                return new Display.Mode(mode.Width, mode.Height, mode.Hz);
            })
            .ToList();
    }
}
