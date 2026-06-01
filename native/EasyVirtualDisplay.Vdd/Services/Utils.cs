using System;
using System.Collections.Generic;
using EasyVirtualDisplay.Vdd.Domain;
using Microsoft.Win32;

namespace EasyVirtualDisplay.Vdd.Services;

internal static class Utils
{
    public static IList<Display.Mode> GetCustomDisplayModes()
    {
        var list = new List<Display.Mode>();

        using (var vdd = Registry.LocalMachine.OpenSubKey("SOFTWARE\\Parsec\\vdd", RegistryKeyPermissionCheck.ReadSubTree))
        {
            if (vdd != null)
            {
                for (int i = 0; i < 5; i++)
                {
                    using (var index = vdd.OpenSubKey($"{i}", RegistryKeyPermissionCheck.ReadSubTree))
                    {
                        if (index != null)
                        {
                            var width = index.GetValue("width");
                            var height = index.GetValue("height");
                            var hz = index.GetValue("hz");

                            if (width != null && height != null && hz != null)
                            {
                                list.Add(new Display.Mode
                                {
                                    Width = Convert.ToUInt16(width),
                                    Height = Convert.ToUInt16(height),
                                    Hz = Convert.ToUInt16(hz),
                                });
                            }
                        }
                    }
                }
            }
        }

        return list;
    }

    public static void SetCustomDisplayModes(List<Display.Mode> modes)
    {
        using (var vdd = Registry.LocalMachine.CreateSubKey("SOFTWARE\\Parsec\\vdd", RegistryKeyPermissionCheck.ReadWriteSubTree))
        {
            if (vdd != null)
            {
                for (int i = 0; i < 5; i++)
                {
                    using (var index = vdd.CreateSubKey($"{i}", RegistryKeyPermissionCheck.ReadWriteSubTree))
                    {
                        if (i >= modes.Count && index != null)
                        {
                            index.Dispose();
                            vdd.DeleteSubKey($"{i}");
                        }
                        else if (index != null)
                        {
                            index.SetValue("width", modes[i].Width, RegistryValueKind.DWord);
                            index.SetValue("height", modes[i].Height, RegistryValueKind.DWord);
                            index.SetValue("hz", modes[i].Hz, RegistryValueKind.DWord);
                        }
                    }
                }
            }
        }
    }

    public enum ParentGPU
    {
        Auto = 0,
        NVIDIA = 0x10DE,
        AMD = 0x1002,
    }

    public static ParentGPU GetParentGPU()
    {
        using (var parameters = Registry.LocalMachine.OpenSubKey(
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\WUDF\\Services\\ParsecVDA\\Parameters",
            RegistryKeyPermissionCheck.ReadSubTree))
        {
            if (parameters != null)
            {
                object? value = parameters.GetValue("PreferredRenderAdapterVendorId");
                if (value != null)
                {
                    return (ParentGPU)Convert.ToInt32(value);
                }
            }
        }

        return ParentGPU.Auto;
    }

    public static void SetParentGPU(ParentGPU kind)
    {
        using (var parameters = Registry.LocalMachine.OpenSubKey(
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\WUDF\\Services\\ParsecVDA\\Parameters",
            RegistryKeyPermissionCheck.ReadWriteSubTree))
        {
            if (parameters != null)
            {
                if (kind == ParentGPU.Auto)
                {
                    parameters.DeleteValue("PreferredRenderAdapterVendorId", false);
                }
                else
                {
                    parameters.SetValue("PreferredRenderAdapterVendorId",
                        (uint)kind, RegistryValueKind.DWord);
                }
            }
        }
    }
}
