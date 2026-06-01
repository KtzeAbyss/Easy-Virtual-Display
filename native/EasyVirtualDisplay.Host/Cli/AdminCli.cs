using System.Diagnostics;
using System.IO;
using System.Text.Json;
using EasyVirtualDisplay.Host.Hosting;
using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Services;
using Microsoft.Win32;

namespace EasyVirtualDisplay.Host.Cli;

public static class AdminCli
{
    private const string ApplyAdminConfigCommand = "apply-admin-config";
    private const string InstallDriverCommand = "install-driver";
    private const string UninstallDriverCommand = "uninstall-driver";
    private const string ParsecVddUninstallRegistryPath =
        @"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ParsecVDD";
    private const string ParsecVddPresetsRegistryPath = @"SOFTWARE\Parsec\vdd";

    public static bool CanHandle(IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            return false;
        }

        return string.Equals(args[0], ApplyAdminConfigCommand, StringComparison.OrdinalIgnoreCase)
            || string.Equals(args[0], InstallDriverCommand, StringComparison.OrdinalIgnoreCase)
            || string.Equals(args[0], UninstallDriverCommand, StringComparison.OrdinalIgnoreCase);
    }

    public static Task<int> RunAsync(IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            throw new ArgumentException("Missing administrator command.");
        }

        var command = args[0];

        if (string.Equals(command, ApplyAdminConfigCommand, StringComparison.OrdinalIgnoreCase))
        {
            return RunApplyAdminConfigAsync(args);
        }

        if (string.Equals(command, InstallDriverCommand, StringComparison.OrdinalIgnoreCase))
        {
            return RunInstallDriverAsync(args);
        }

        if (string.Equals(command, UninstallDriverCommand, StringComparison.OrdinalIgnoreCase))
        {
            return RunUninstallDriverAsync(args);
        }

        throw new ArgumentException($"Unknown administrator command '{command}'.");
    }

    private static Task<int> RunApplyAdminConfigAsync(IReadOnlyList<string> args)
    {
        var input = ParseApplyAdminConfigArgs(args);
        var configurationService = new VddConfigurationService();

        configurationService.Apply(input);

        var snapshot = configurationService.GetSnapshot();
        Console.Out.WriteLine(JsonSerializer.Serialize(snapshot, HostJsonOptions.Default));

        return Task.FromResult(0);
    }

    private static async Task<int> RunInstallDriverAsync(IReadOnlyList<string> args)
    {
        var installerPath = ParseInstallDriverArgs(args);
        var result = await InstallDriverAsync(installerPath);

        Console.Out.WriteLine(JsonSerializer.Serialize(result, HostJsonOptions.Default));
        return 0;
    }

    private static async Task<int> RunUninstallDriverAsync(IReadOnlyList<string> args)
    {
        var silent = args.Any(a => string.Equals(a, "--silent", StringComparison.OrdinalIgnoreCase));
        var elevatedFlag = args.Any(a => string.Equals(a, "--elevated", StringComparison.OrdinalIgnoreCase));

        if (!IsProcessElevated())
        {
            if (elevatedFlag)
            {
                throw new ErrorOperationFailed(
                    ErrorOperationFailed.Operation.Uninstall,
                    "driver_uninstall_failed",
                    "The elevated uninstaller process is not running as administrator.");
            }

            return await RunSelfElevatedUninstallAsync(args);
        }

        var result = await UninstallDriverAsync(silent);
        Console.Out.WriteLine(JsonSerializer.Serialize(result, HostJsonOptions.Default));
        return 0;
    }

    private static ApplyAdminConfigInput ParseApplyAdminConfigArgs(IReadOnlyList<string> args)
    {
        string? modesPayload = null;
        string? parentGpu = null;

        for (var index = 1; index < args.Count; index++)
        {
            switch (args[index])
            {
                case "--modes":
                    modesPayload = ReadOptionValue(args, ++index, "--modes");
                    break;

                case "--parent-gpu":
                    parentGpu = ReadOptionValue(args, ++index, "--parent-gpu");
                    break;

                default:
                    throw new ArgumentException($"Unknown argument '{args[index]}'.");
            }
        }

        if (modesPayload is null)
        {
            throw new ArgumentException("Missing --modes payload.");
        }

        if (parentGpu is null)
        {
            throw new ArgumentException("Missing --parent-gpu value.");
        }

        var customModes = JsonSerializer.Deserialize<DisplayMode[]>(modesPayload, HostJsonOptions.Default)
            ?? Array.Empty<DisplayMode>();

        return new ApplyAdminConfigInput
        {
            CustomModes = customModes,
            ParentGpu = parentGpu,
        };
    }

    private static string ParseInstallDriverArgs(IReadOnlyList<string> args)
    {
        string? installerPath = null;

        for (var index = 1; index < args.Count; index++)
        {
            switch (args[index])
            {
                case "--installer-path":
                    installerPath = ReadOptionValue(args, ++index, "--installer-path");
                    break;

                default:
                    throw new ArgumentException($"Unknown argument '{args[index]}'.");
            }
        }

        if (installerPath is null)
        {
            throw new ArgumentException("Missing --installer-path value.");
        }

        return installerPath;
    }

    private static async Task<DriverInstallationResult> InstallDriverAsync(string installerPath)
    {
        var resolvedPath = Path.GetFullPath(installerPath);
        if (!File.Exists(resolvedPath))
        {
            throw new FileNotFoundException("Bundled driver installer was not found.", resolvedPath);
        }

        var startInfo = new ProcessStartInfo
        {
            FileName = resolvedPath,
            Arguments = "/S",
            WorkingDirectory = Path.GetDirectoryName(resolvedPath) ?? Environment.CurrentDirectory,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };

        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Failed to start the bundled driver installer.");

        var stdoutTask = process.StandardOutput.ReadToEndAsync();
        var stderrTask = process.StandardError.ReadToEndAsync();

        process.WaitForExit();

        var stdout = stdoutTask.GetAwaiter().GetResult().Trim();
        var stderr = stderrTask.GetAwaiter().GetResult().Trim();

        if (process.ExitCode != 0)
        {
            var detail = string.IsNullOrWhiteSpace(stderr) ? stdout : stderr;
            throw new InvalidOperationException(
                string.IsNullOrWhiteSpace(detail)
                    ? $"Bundled driver installer failed with exit code {process.ExitCode}."
                    : $"Bundled driver installer failed with exit code {process.ExitCode}: {detail}");
        }

        await using var session = new VddSession();
        var snapshot = await session.GetSnapshotAsync();

        if (string.Equals(snapshot.Status, DriverStatuses.NotInstalled, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException(
                "Bundled driver installer completed but the virtual display driver is still not installed.");
        }

        return new DriverInstallationResult
        {
            Status = snapshot.Status,
            DriverVersion = snapshot.DriverVersion,
        };
    }

    private static bool IsProcessElevated()
    {
        using var identity = System.Security.Principal.WindowsIdentity.GetCurrent();
        var principal = new System.Security.Principal.WindowsPrincipal(identity);
        return principal.IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator);
    }

    private static async Task<int> RunSelfElevatedUninstallAsync(IReadOnlyList<string> args)
    {
        var self = Environment.ProcessPath
            ?? throw new InvalidOperationException("Unable to determine host executable path.");

        var startInfo = new ProcessStartInfo
        {
            FileName = self,
            UseShellExecute = true,
            Verb = "runas",
            CreateNoWindow = true,
            WindowStyle = ProcessWindowStyle.Hidden,
        };

        foreach (var arg in args)
        {
            startInfo.ArgumentList.Add(arg);
        }

        startInfo.ArgumentList.Add("--elevated");

        try
        {
            using var process = Process.Start(startInfo)
                ?? throw new InvalidOperationException("Failed to elevate the uninstaller process.");
            await process.WaitForExitAsync();
            return process.ExitCode;
        }
        catch (System.ComponentModel.Win32Exception ex) when (ex.NativeErrorCode == 1223)
        {
            throw new ErrorOperationFailed(
                ErrorOperationFailed.Operation.Uninstall,
                "admin_cancelled",
                "Administrator approval was cancelled.");
        }
    }

    private static async Task<DriverUninstallationResult> UninstallDriverAsync(bool silent)
    {
        var uninstallEntry = ReadParsecVddUninstallEntry();
        if (uninstallEntry is null)
        {
            throw new ErrorOperationFailed(
                ErrorOperationFailed.Operation.Uninstall,
                "driver_uninstall_not_installed",
                "No Parsec Virtual Display Driver entry found in the registry.");
        }

        await RunSilentUninstallerAsync(uninstallEntry);
        RemoveParsecVddPresetsRegistry();

        return new DriverUninstallationResult
        {
            Status = "uninstalled",
            DriverVersion = uninstallEntry.DisplayVersion ?? "0.0.0.0",
        };
    }

    private static ParsecVddUninstallEntry? ReadParsecVddUninstallEntry()
    {
        using var baseKey = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64);
        using var key = baseKey.OpenSubKey(ParsecVddUninstallRegistryPath, writable: false);
        if (key is null) return null;

        var uninstallString = key.GetValue("UninstallString") as string;
        if (string.IsNullOrWhiteSpace(uninstallString)) return null;

        var exe = uninstallString.Trim().Trim('"');
        if (!File.Exists(exe)) return null;

        var installLocation = (key.GetValue("InstallLocation") as string)?.Trim().Trim('"')
            ?? Path.GetDirectoryName(exe)
            ?? throw new InvalidOperationException("Unable to resolve Parsec VDD install location.");

        var displayVersion = key.GetValue("DisplayVersion") as string;
        return new ParsecVddUninstallEntry(exe, installLocation, displayVersion);
    }

    private static async Task RunSilentUninstallerAsync(ParsecVddUninstallEntry entry)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = entry.UninstallExe,
            Arguments = $"/S _?={entry.InstallLocation}",
            WorkingDirectory = entry.InstallLocation,
            UseShellExecute = true,
            Verb = "runas",
            CreateNoWindow = true,
            WindowStyle = ProcessWindowStyle.Hidden,
        };

        Process? process;
        try
        {
            process = Process.Start(startInfo);
        }
        catch (System.ComponentModel.Win32Exception ex) when (ex.NativeErrorCode == 1223)
        {
            throw new ErrorOperationFailed(
                ErrorOperationFailed.Operation.Uninstall,
                "admin_cancelled",
                "Administrator approval was cancelled.");
        }

        if (process is null)
        {
            throw new InvalidOperationException("Failed to start the Parsec VDD uninstaller.");
        }

        using (process)
        {
            await process.WaitForExitAsync();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException(
                    $"Parsec VDD uninstaller exited with code {process.ExitCode}.");
            }
        }
    }

    private static void RemoveParsecVddPresetsRegistry()
    {
        using var baseKey = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64);
        baseKey.DeleteSubKeyTree(ParsecVddPresetsRegistryPath, throwOnMissingSubKey: false);
    }

    private static string ReadOptionValue(IReadOnlyList<string> args, int index, string optionName)
    {
        if (index >= args.Count)
        {
            throw new ArgumentException($"Missing value for {optionName}.");
        }

        return args[index];
    }

    private sealed class DriverInstallationResult
    {
        public string Status { get; init; } = DriverStatuses.Unknown;

        public string DriverVersion { get; init; } = "0.0.0.0";
    }

    private sealed record ParsecVddUninstallEntry(
        string UninstallExe,
        string InstallLocation,
        string? DisplayVersion);

    private sealed class DriverUninstallationResult
    {
        public string Status { get; init; } = "uninstalled";

        public string DriverVersion { get; init; } = "0.0.0.0";
    }
}
