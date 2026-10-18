using System.Diagnostics;
using System.Text;
using System.Text.Json;
using EasyVirtualDisplay.Host.Hosting;

namespace EasyVirtualDisplay.Host.Control;

/// <summary>
/// `EasyVirtualDisplay.Host.exe ctl ...`: a short-lived client that asks the running app
/// to add or remove a display and exits with the outcome, starting the app in the
/// background first when it is not running.
/// </summary>
internal static class ControlCli
{
    public const string Command = "ctl";

    private const int ExitOk = 0;
    private const int ExitUsage = 1;
    private const int ExitUnavailable = 2;
    private const int ExitFailed = 3;
    private const int ExitTimeout = 4;

    private const int DefaultWaitSeconds = 15;
    private const string AppExecutable = "EasyVirtualDisplay.exe";
    private const int ConnectTimeoutMs = 500;

    // Covers a cold start: the app launching, then starting the host.
    private const int StartupConnectTimeoutMs = 30_000;

    // Slack on top of a requested wait for the add itself and the round trip.
    private static readonly TimeSpan ResponseSlack = TimeSpan.FromSeconds(30);

    private const string Usage = """
        Usage:
          evdctl ensure --key <key> [--wait [seconds]]
              Add a virtual display owned by <key>, unless it already has one.
              --wait also waits until that display is active (default 15 seconds).
          evdctl release --key <key>
              Remove the display owned by <key>. Succeeds when there is none.
          evdctl status
              Print the driver status and displays as JSON.

        Exit codes: 0 success, 1 invalid usage, 2 Easy Virtual Display unavailable,
        3 operation failed, 4 timed out waiting for the display.
        """;

    public static bool CanHandle(IReadOnlyList<string> args)
    {
        return args.Count > 0 && string.Equals(args[0], Command, StringComparison.OrdinalIgnoreCase);
    }

    public static async Task<int> RunAsync(IReadOnlyList<string> args)
    {
        Console.OutputEncoding = Encoding.UTF8;

        ControlRequest request;
        try
        {
            request = Parse(args.Skip(1).ToList());
        }
        catch (ArgumentException ex)
        {
            await Console.Error.WriteLineAsync(ex.Message);
            await Console.Error.WriteLineAsync(Usage);
            return ExitUsage;
        }

        await using var pipe = ControlPipe.CreateClient();
        var unavailable = await ConnectAsync(pipe);
        if (unavailable is not null)
        {
            await Console.Error.WriteLineAsync(unavailable);
            return ExitUnavailable;
        }

        ControlResponse? response;
        try
        {
            using var timeout = new CancellationTokenSource(
                TimeSpan.FromSeconds(request.WaitSeconds ?? 0) + ResponseSlack);
            response = await SendAsync(pipe, request, timeout.Token);
        }
        catch (Exception ex) when (ex is IOException or OperationCanceledException or JsonException)
        {
            await Console.Error.WriteLineAsync($"No answer from Easy Virtual Display: {ex.Message}");
            return ExitUnavailable;
        }

        return await ReportAsync(request, response);
    }

    private static ControlRequest Parse(IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            throw new ArgumentException("Missing command.");
        }

        var command = args[0].ToLowerInvariant();
        string? key = null;
        int? waitSeconds = null;

        for (var i = 1; i < args.Count; i++)
        {
            switch (args[i].ToLowerInvariant())
            {
                case "--key" when i + 1 < args.Count:
                    key = args[++i];
                    break;

                case "--wait":
                    waitSeconds = DefaultWaitSeconds;
                    if (i + 1 < args.Count && int.TryParse(args[i + 1], out var seconds))
                    {
                        waitSeconds = seconds;
                        i++;
                    }

                    break;

                default:
                    throw new ArgumentException($"Unexpected argument '{args[i]}'.");
            }
        }

        switch (command)
        {
            case ControlCommands.Ensure:
                RequireKey(key);
                break;

            case ControlCommands.Release:
            case ControlCommands.Status:
                if (waitSeconds is not null)
                {
                    throw new ArgumentException($"--wait only applies to {ControlCommands.Ensure}.");
                }

                if (command == ControlCommands.Release)
                {
                    RequireKey(key);
                }
                else if (key is not null)
                {
                    throw new ArgumentException($"{ControlCommands.Status} takes no --key.");
                }

                break;

            default:
                throw new ArgumentException($"Unknown command '{args[0]}'.");
        }

        return new ControlRequest { Command = command, Key = key, WaitSeconds = waitSeconds };
    }

    private static void RequireKey(string? key)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            throw new ArgumentException("--key <key> is required.");
        }
    }

    /// <summary>Connects, starting the app when nothing answers. Returns an error message on failure.</summary>
    private static async Task<string?> ConnectAsync(System.IO.Pipes.NamedPipeClientStream pipe)
    {
        try
        {
            await pipe.ConnectAsync(ConnectTimeoutMs);
            return null;
        }
        catch (TimeoutException)
        {
        }

        var app = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", AppExecutable));
        if (!File.Exists(app))
        {
            return $"Easy Virtual Display is not running, and {app} was not found to start it.";
        }

        try
        {
            // Shell execute so the app inherits none of our handles: a caller waiting for
            // our output to close (as streaming hosts do) must not wait on the app too.
            Process.Start(new ProcessStartInfo(app, "--background")
            {
                UseShellExecute = true,
                WorkingDirectory = Path.GetDirectoryName(app),
            })?.Dispose();
        }
        catch (Exception ex)
        {
            return $"Could not start Easy Virtual Display: {ex.Message}";
        }

        try
        {
            await pipe.ConnectAsync(StartupConnectTimeoutMs);
            return null;
        }
        catch (TimeoutException)
        {
            return $"Easy Virtual Display did not become ready within {StartupConnectTimeoutMs / 1000}s.";
        }
    }

    private static async Task<ControlResponse?> SendAsync(
        Stream pipe,
        ControlRequest request,
        CancellationToken cancellationToken)
    {
        await using var writer = new StreamWriter(pipe, new UTF8Encoding(false), 1024, leaveOpen: true)
        {
            NewLine = "\n",
        };
        using var reader = new StreamReader(pipe, new UTF8Encoding(false), false, 1024, leaveOpen: true);

        await writer.WriteLineAsync(JsonSerializer.Serialize(request, HostJsonOptions.Default));
        await writer.FlushAsync(cancellationToken);

        var line = await reader.ReadLineAsync(cancellationToken);
        return line is null ? null : JsonSerializer.Deserialize<ControlResponse>(line, HostJsonOptions.Default);
    }

    private static async Task<int> ReportAsync(ControlRequest request, ControlResponse? response)
    {
        if (response is null)
        {
            await Console.Error.WriteLineAsync("Easy Virtual Display closed the connection without answering.");
            return ExitUnavailable;
        }

        if (!response.Ok)
        {
            await Console.Error.WriteLineAsync($"{response.Code}: {response.Message}");
            return response.Code switch
            {
                ControlErrorCodes.Timeout => ExitTimeout,
                ControlErrorCodes.InvalidRequest => ExitUsage,
                _ => ExitFailed,
            };
        }

        switch (request.Command)
        {
            case ControlCommands.Ensure:
                Console.WriteLine(request.WaitSeconds is null
                    ? $"Display {response.Index} is assigned to '{request.Key}'."
                    : $"Display {response.Index} is assigned to '{request.Key}' and active.");
                break;

            case ControlCommands.Release:
                Console.WriteLine(response.Released == true
                    ? $"Released the display of '{request.Key}'."
                    : $"No display was assigned to '{request.Key}'.");
                break;

            case ControlCommands.Status:
                Console.WriteLine(JsonSerializer.Serialize(
                    response.Status,
                    new JsonSerializerOptions(HostJsonOptions.Default) { WriteIndented = true }));
                break;
        }

        return ExitOk;
    }
}
