using System.Text;
using System.Text.Json;
using EasyVirtualDisplay.Host.Errors;
using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Services;

namespace EasyVirtualDisplay.Host.Hosting;

public static class StdioHost
{
    public static async Task<int> RunAsync(IReadOnlyList<string> args)
    {
        ValidateArguments(args);

        Console.InputEncoding = Encoding.UTF8;
        Console.OutputEncoding = Encoding.UTF8;

        await using var session = new VddSession();
        await session.StartAsync();

        await using var rpcHost = new StdioRpcHost(session);
        return await rpcHost.RunAsync();
    }

    private static void ValidateArguments(IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            return;
        }

        if (args.Count == 1 && string.Equals(args[0], "--stdio", StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        throw new ArgumentException($"Unsupported stdio host arguments: {string.Join(' ', args)}");
    }
}

internal sealed class StdioRpcHost : IAsyncDisposable
{
    private const string JsonRpcVersion = "2.0";

    private readonly VddSession _session;
    private readonly StreamReader _reader;
    private readonly StreamWriter _writer;
    private readonly SemaphoreSlim _writeLock = new(1, 1);

    public StdioRpcHost(VddSession session)
    {
        _session = session;
        _reader = new StreamReader(Console.OpenStandardInput(), Encoding.UTF8, false);
        _writer = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false))
        {
            AutoFlush = true,
            NewLine = "\n",
        };

        _session.SnapshotChanged += HandleSnapshotChanged;
    }

    public async Task<int> RunAsync(CancellationToken cancellationToken = default)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            var line = await _reader.ReadLineAsync(cancellationToken);
            if (line is null)
            {
                return 0;
            }

            if (string.IsNullOrWhiteSpace(line))
            {
                continue;
            }

            await HandleLineAsync(line, cancellationToken);
        }

        return 0;
    }

    public ValueTask DisposeAsync()
    {
        _session.SnapshotChanged -= HandleSnapshotChanged;
        _writer.Dispose();
        _reader.Dispose();
        _writeLock.Dispose();
        return ValueTask.CompletedTask;
    }

    private void HandleSnapshotChanged(object? sender, HostSnapshot snapshot)
    {
        _ = WriteMessageAsync(
            new
            {
                jsonrpc = JsonRpcVersion,
                method = "host.snapshotChanged",
                @params = snapshot,
            },
            CancellationToken.None);
    }

    private async Task HandleLineAsync(string line, CancellationToken cancellationToken)
    {
        JsonDocument document;

        try
        {
            document = JsonDocument.Parse(line);
        }
        catch (JsonException ex)
        {
            await WriteStandardErrorAsync(null, -32700, "Parse error", ex.Message, cancellationToken);
            return;
        }

        using (document)
        {
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
            {
                await WriteStandardErrorAsync(null, -32600, "Invalid Request", "Request body must be a JSON object.", cancellationToken);
                return;
            }

            var id = root.TryGetProperty("id", out var idElement)
                ? idElement.Clone()
                : (JsonElement?)null;

            if (!root.TryGetProperty("method", out var methodElement)
                || methodElement.ValueKind != JsonValueKind.String)
            {
                await WriteStandardErrorAsync(id, -32600, "Invalid Request", "Missing JSON-RPC method.", cancellationToken);
                return;
            }

            var method = methodElement.GetString()!;
            var hasParams = root.TryGetProperty("params", out var paramsElement);

            try
            {
                var result = await DispatchAsync(method, paramsElement, hasParams, cancellationToken);
                if (id.HasValue)
                {
                    await WriteMessageAsync(
                        new
                        {
                            jsonrpc = JsonRpcVersion,
                            id = id.Value,
                            result,
                        },
                        cancellationToken);
                }
            }
            catch (NotSupportedException ex)
            {
                if (id.HasValue)
                {
                    await WriteStandardErrorAsync(id, -32601, "Method not found", ex.Message, cancellationToken);
                }
            }
            catch (Exception ex)
            {
                if (id.HasValue)
                {
                    var hostError = HostErrorMapper.Normalize(ex);
                    await WriteMessageAsync(
                        new
                        {
                            jsonrpc = JsonRpcVersion,
                            id = id.Value,
                            error = new
                            {
                                code = -32000,
                                message = hostError.Message,
                                data = hostError,
                            },
                        },
                        cancellationToken);
                }
            }
        }
    }

    private async Task<object?> DispatchAsync(
        string method,
        JsonElement paramsElement,
        bool hasParams,
        CancellationToken cancellationToken)
    {
        switch (method)
        {
            case "host.getSnapshot":
                return await _session.GetSnapshotAsync(cancellationToken);

            case "host.addDisplay":
                await _session.AddDisplayAsync(cancellationToken);
                return null;

            case "host.removeDisplay":
                int? index = null;
                if (hasParams)
                {
                    if (paramsElement.ValueKind != JsonValueKind.Object)
                    {
                        throw new ArgumentException("host.removeDisplay params must be a JSON object.");
                    }

                    if (paramsElement.TryGetProperty("index", out var indexElement)
                        && indexElement.ValueKind != JsonValueKind.Null)
                    {
                        index = indexElement.GetInt32();
                    }
                }

                await _session.RemoveDisplayAsync(index, cancellationToken);
                return null;

            case "host.removeAllDisplays":
                await _session.RemoveAllDisplaysAsync(cancellationToken);
                return null;

            case "host.setDisplayMode":
                if (!hasParams || paramsElement.ValueKind != JsonValueKind.Object)
                {
                    throw new ArgumentException("host.setDisplayMode params must be a JSON object.");
                }

                var input = paramsElement.Deserialize<SetDisplayModeInput>(HostJsonOptions.Default)
                    ?? throw new ArgumentException("host.setDisplayMode params must be a JSON object.");

                await _session.SetDisplayModeAsync(input, cancellationToken);
                return null;

            default:
                throw new NotSupportedException($"Unsupported method '{method}'.");
        }
    }

    private Task WriteStandardErrorAsync(
        JsonElement? id,
        int code,
        string message,
        string details,
        CancellationToken cancellationToken)
    {
        return WriteMessageAsync(
            new
            {
                jsonrpc = JsonRpcVersion,
                id,
                error = new
                {
                    code,
                    message,
                    data = new
                    {
                        details,
                    },
                },
            },
            cancellationToken);
    }

    private async Task WriteMessageAsync(object payload, CancellationToken cancellationToken)
    {
        var acquired = false;

        try
        {
            await _writeLock.WaitAsync(cancellationToken);
            acquired = true;

            var json = JsonSerializer.Serialize(payload, HostJsonOptions.Default);
            await _writer.WriteLineAsync(json);
            await _writer.FlushAsync();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (ObjectDisposedException)
        {
        }
        finally
        {
            if (acquired)
            {
                _writeLock.Release();
            }
        }
    }
}
