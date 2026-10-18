using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using EasyVirtualDisplay.Host.Errors;
using EasyVirtualDisplay.Host.Hosting;
using EasyVirtualDisplay.Vdd.Services;

namespace EasyVirtualDisplay.Host.Control;

/// <summary>Serves `ctl` commands over <see cref="ControlPipe"/> for as long as the host runs.</summary>
internal sealed class ControlServer : IAsyncDisposable
{
    private const int MaxKeyLength = 64;
    private const int MaxWaitSeconds = 300;
    private static readonly TimeSpan RequestReadTimeout = TimeSpan.FromSeconds(10);

    // A host restarted by the app may briefly overlap the previous one's pipe.
    private const int FirstInstanceAttempts = 10;
    private static readonly TimeSpan FirstInstanceRetryDelay = TimeSpan.FromMilliseconds(500);

    private readonly IDisplayBackend _backend;
    private readonly KeyedDisplays _keyed;
    private readonly CancellationTokenSource _lifetime = new();
    private readonly Task _acceptLoop;

    private ControlServer(IDisplayBackend backend, KeyedDisplays keyed)
    {
        _backend = backend;
        _keyed = keyed;
        _acceptLoop = Task.Run(() => AcceptLoopAsync(_lifetime.Token));
    }

    public static ControlServer Start(IDisplayBackend backend, KeyedDisplays keyed) => new(backend, keyed);

    public async ValueTask DisposeAsync()
    {
        _lifetime.Cancel();
        await _acceptLoop;
        _lifetime.Dispose();
    }

    private async Task AcceptLoopAsync(CancellationToken cancellationToken)
    {
        var pipe = await CreateFirstInstanceAsync(cancellationToken);

        while (pipe is not null)
        {
            try
            {
                await pipe.WaitForConnectionAsync(cancellationToken);
            }
            catch (OperationCanceledException)
            {
                await pipe.DisposeAsync();
                return;
            }

            var connected = pipe;
            _ = Task.Run(() => ServeAsync(connected, cancellationToken), CancellationToken.None);

            try
            {
                pipe = ControlPipe.CreateServer(firstInstance: false);
            }
            catch (Exception ex)
            {
                await Console.Error.WriteLineAsync($"Control pipe stopped: {ex.Message}");
                return;
            }
        }
    }

    private static async Task<NamedPipeServerStream?> CreateFirstInstanceAsync(CancellationToken cancellationToken)
    {
        for (var attempt = 1; ; attempt++)
        {
            try
            {
                return ControlPipe.CreateServer(firstInstance: true);
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                if (attempt >= FirstInstanceAttempts)
                {
                    await Console.Error.WriteLineAsync($"Control pipe unavailable: {ex.Message}");
                    return null;
                }
            }

            try
            {
                await Task.Delay(FirstInstanceRetryDelay, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                return null;
            }
        }
    }

    private async Task ServeAsync(NamedPipeServerStream pipe, CancellationToken cancellationToken)
    {
        await using (pipe)
        {
            try
            {
                using var reader = new StreamReader(pipe, new UTF8Encoding(false), false, 1024, leaveOpen: true);
                await using var writer = new StreamWriter(pipe, new UTF8Encoding(false), 1024, leaveOpen: true)
                {
                    NewLine = "\n",
                };

                string? line;
                using (var readTimeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken))
                {
                    readTimeout.CancelAfter(RequestReadTimeout);
                    line = await reader.ReadLineAsync(readTimeout.Token);
                }

                var response = await ExecuteAsync(line, cancellationToken);
                await writer.WriteLineAsync(JsonSerializer.Serialize(response, HostJsonOptions.Default));
                await writer.FlushAsync(cancellationToken);
            }
            catch (Exception ex) when (ex is IOException or OperationCanceledException or ObjectDisposedException)
            {
                // The client went away or the host is shutting down; nobody is left to answer.
            }
        }
    }

    private async Task<ControlResponse> ExecuteAsync(string? line, CancellationToken cancellationToken)
    {
        ControlRequest? request;
        try
        {
            request = line is null ? null : JsonSerializer.Deserialize<ControlRequest>(line, HostJsonOptions.Default);
        }
        catch (JsonException ex)
        {
            return ControlResponse.Failure(ControlErrorCodes.InvalidRequest, ex.Message);
        }

        if (request is null)
        {
            return ControlResponse.Failure(ControlErrorCodes.InvalidRequest, "Empty request.");
        }

        try
        {
            switch (request.Command)
            {
                case ControlCommands.Ensure:
                {
                    var key = ValidateKey(request.Key);
                    if (request.WaitSeconds is < 0 or > MaxWaitSeconds)
                    {
                        throw new ArgumentException($"waitSeconds must be between 0 and {MaxWaitSeconds}.");
                    }

                    TimeSpan? wait = request.WaitSeconds is null ? null : TimeSpan.FromSeconds(request.WaitSeconds.Value);
                    var index = await _keyed.EnsureAsync(key, wait, cancellationToken);
                    return new ControlResponse { Ok = true, Index = index };
                }

                case ControlCommands.Release:
                {
                    var released = await _keyed.ReleaseAsync(ValidateKey(request.Key), cancellationToken);
                    return new ControlResponse { Ok = true, Released = released };
                }

                case ControlCommands.Status:
                    return new ControlResponse { Ok = true, Status = await BuildStatusAsync(cancellationToken) };

                default:
                    return ControlResponse.Failure(ControlErrorCodes.InvalidRequest, $"Unknown command '{request.Command}'.");
            }
        }
        catch (ArgumentException ex)
        {
            return ControlResponse.Failure(ControlErrorCodes.InvalidRequest, ex.Message);
        }
        catch (TimeoutException ex)
        {
            return ControlResponse.Failure(ControlErrorCodes.Timeout, ex.Message);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            var error = HostErrorMapper.Normalize(ex);
            return ControlResponse.Failure(error.Code, error.Message);
        }
    }

    private async Task<ControlStatus> BuildStatusAsync(CancellationToken cancellationToken)
    {
        var snapshot = await _backend.GetSnapshotAsync(cancellationToken);
        var owners = new Dictionary<int, string>();
        foreach (var (key, index) in _keyed.GetBindings())
        {
            owners.TryAdd(index, key);
        }

        return new ControlStatus
        {
            DriverStatus = snapshot.Status,
            Displays = snapshot.Displays
                .Select(display => new ControlDisplay
                {
                    Index = display.Index,
                    Active = display.Active,
                    Key = owners.GetValueOrDefault(display.Index),
                })
                .ToArray(),
        };
    }

    private static string ValidateKey(string? key)
    {
        if (string.IsNullOrWhiteSpace(key) || key.Length > MaxKeyLength)
        {
            throw new ArgumentException($"A key of 1 to {MaxKeyLength} characters is required.");
        }

        return key;
    }
}
