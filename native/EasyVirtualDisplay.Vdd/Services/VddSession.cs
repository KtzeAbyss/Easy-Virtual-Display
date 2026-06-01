using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Interop;

namespace EasyVirtualDisplay.Vdd.Services;

public sealed class VddSession : IAsyncDisposable
{
    private static readonly TimeSpan KeepAliveInterval = TimeSpan.FromMilliseconds(100);
    private static readonly TimeSpan HealthCheckInterval = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan SnapshotDiffInterval = TimeSpan.FromMilliseconds(750);

    private readonly SemaphoreSlim _gate = new(1, 1);
    private readonly VddConfigurationService _configurationService;

    private CancellationTokenSource? _lifetimeCts;
    private Task? _keepAliveTask;
    private Task? _healthCheckTask;
    private Task? _snapshotDiffTask;

    private HostSnapshot _snapshot = ContractMapper.CreateEmptySnapshot();
    private string _snapshotFingerprint = string.Empty;
    private Device.Status _status = Device.Status.UNKNOWN;
    private Version _driverVersion = new(0, 0, 0, 0);
    private IntPtr _handle = IntPtr.Zero;
    private bool _started;
    private bool _disposed;

    public VddSession()
        : this(new VddConfigurationService())
    {
    }

    internal VddSession(VddConfigurationService configurationService)
    {
        _configurationService = configurationService;
    }

    public event EventHandler<HostSnapshot>? SnapshotChanged;

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        ThrowIfDisposed();

        await _gate.WaitAsync(cancellationToken);

        try
        {
            if (_started)
            {
                return;
            }

            _started = true;
            _lifetimeCts = new CancellationTokenSource();

            RefreshDriverStateCore();
            UpdateSnapshotCore(emitIfChanged: false);

            _keepAliveTask = RunPeriodicLoopAsync(KeepAliveInterval, RunKeepAliveOnceAsync, _lifetimeCts.Token);
            _healthCheckTask = RunPeriodicLoopAsync(HealthCheckInterval, RunHealthCheckOnceAsync, _lifetimeCts.Token);
            _snapshotDiffTask = RunPeriodicLoopAsync(SnapshotDiffInterval, RunSnapshotDiffOnceAsync, _lifetimeCts.Token);
        }
        finally
        {
            _gate.Release();
        }
    }

    public async Task StopAsync()
    {
        CancellationTokenSource? lifetimeCts;
        Task[] tasks;

        await _gate.WaitAsync();

        try
        {
            if (!_started)
            {
                return;
            }

            _started = false;
            lifetimeCts = _lifetimeCts;
            _lifetimeCts = null;

            tasks = new[]
            {
                _keepAliveTask,
                _healthCheckTask,
                _snapshotDiffTask,
            }
            .Where(task => task is not null)
            .Cast<Task>()
            .ToArray();

            _keepAliveTask = null;
            _healthCheckTask = null;
            _snapshotDiffTask = null;
        }
        finally
        {
            _gate.Release();
        }

        lifetimeCts?.Cancel();

        if (tasks.Length > 0)
        {
            try
            {
                await Task.WhenAll(tasks);
            }
            catch (OperationCanceledException)
            {
            }
        }

        lifetimeCts?.Dispose();

        await _gate.WaitAsync();

        try
        {
            CloseHandleCore();
        }
        finally
        {
            _gate.Release();
        }
    }

    public async Task<HostSnapshot> GetSnapshotAsync(CancellationToken cancellationToken = default)
    {
        await StartAsync(cancellationToken);

        await _gate.WaitAsync(cancellationToken);

        try
        {
            return _snapshot;
        }
        finally
        {
            _gate.Release();
        }
    }

    public async Task AddDisplayAsync(CancellationToken cancellationToken = default)
    {
        await StartAsync(cancellationToken);

        await _gate.WaitAsync(cancellationToken);

        try
        {
            EnsureDriverReadyCore(requireHandle: true);

            var displays = Core.GetDisplays();
            if (displays.Count >= Core.MAX_DISPLAYS)
            {
                throw new ErrorExceededLimit(Core.MAX_DISPLAYS);
            }

            if (!Core.AddDisplay(_handle, out _))
            {
                throw new ErrorOperationFailed(ErrorOperationFailed.Operation.AddDisplay);
            }
        }
        finally
        {
            _gate.Release();
        }

        await RefreshSnapshotAsync(cancellationToken);
    }

    public async Task RemoveDisplayAsync(int? index = null, CancellationToken cancellationToken = default)
    {
        await StartAsync(cancellationToken);

        var changed = false;

        await _gate.WaitAsync(cancellationToken);

        try
        {
            EnsureDriverReadyCore(requireHandle: true);

            var displays = Core.GetDisplays();
            var targetDisplay = ResolveDisplay(displays, index);
            if (targetDisplay is null)
            {
                return;
            }

            if (!Core.RemoveDisplay(_handle, targetDisplay.DisplayIndex))
            {
                throw new ErrorOperationFailed(ErrorOperationFailed.Operation.RemoveDisplay);
            }

            changed = true;
        }
        finally
        {
            _gate.Release();
        }

        if (changed)
        {
            await RefreshSnapshotAsync(cancellationToken);
        }
    }

    public async Task RemoveAllDisplaysAsync(CancellationToken cancellationToken = default)
    {
        await StartAsync(cancellationToken);

        var changed = false;

        await _gate.WaitAsync(cancellationToken);

        try
        {
            EnsureDriverReadyCore(requireHandle: true);

            var displays = Core.GetDisplays();

            for (var i = displays.Count - 1; i >= 0; i--)
            {
                if (!Core.RemoveDisplay(_handle, displays[i].DisplayIndex))
                {
                    throw new ErrorOperationFailed(ErrorOperationFailed.Operation.RemoveDisplay);
                }

                changed = true;
            }
        }
        finally
        {
            _gate.Release();
        }

        if (changed)
        {
            await RefreshSnapshotAsync(cancellationToken);
        }
    }

    public async Task SetDisplayModeAsync(SetDisplayModeInput input, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(input);

        if (input.Width.HasValue != input.Height.HasValue)
        {
            throw new ArgumentException("Width and height must be provided together.");
        }

        await StartAsync(cancellationToken);

        var changed = false;

        await _gate.WaitAsync(cancellationToken);

        try
        {
            EnsureDriverReadyCore(requireHandle: false);

            var display = Core.GetDisplays().FirstOrDefault(candidate => candidate.DisplayIndex == input.Index);
            if (display is null)
            {
                throw new ErrorDisplayNotFound(input.Index);
            }

            Display.Orientation? orientation = input.Orientation is null
                ? null
                : ContractMapper.ParseOrientation(input.Orientation);

            if (input.Width is null && input.Height is null && input.Hz is null && orientation is null)
            {
                return;
            }

            if (!display.ChangeMode(input.Width, input.Height, input.Hz, orientation))
            {
                throw new ErrorUnsupportedMode(input.Index, input.Width, input.Height, input.Hz, input.Orientation);
            }

            changed = true;
        }
        finally
        {
            _gate.Release();
        }

        if (changed)
        {
            await RefreshSnapshotAsync(cancellationToken);
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        await StopAsync();
        _gate.Dispose();
    }

    private static async Task RunPeriodicLoopAsync(
        TimeSpan interval,
        Func<CancellationToken, Task> tick,
        CancellationToken cancellationToken)
    {
        using var timer = new PeriodicTimer(interval);

        try
        {
            while (await timer.WaitForNextTickAsync(cancellationToken))
            {
                try
                {
                    await tick(cancellationToken);
                }
                catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                {
                    break;
                }
                catch (Exception ex)
                {
                    Debug.WriteLine(ex);
                }
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
    }

    private async Task RunKeepAliveOnceAsync(CancellationToken cancellationToken)
    {
        await Task.Yield();

        if (cancellationToken.IsCancellationRequested)
        {
            return;
        }

        var status = _status;
        var handle = _handle;

        if (status != Device.Status.OK || !handle.IsValidHandle())
        {
            return;
        }

        _ = Core.Update(handle);
    }

    private async Task RunHealthCheckOnceAsync(CancellationToken cancellationToken)
    {
        await _gate.WaitAsync(cancellationToken);

        try
        {
            RefreshDriverStateCore();
        }
        finally
        {
            _gate.Release();
        }
    }

    private Task RunSnapshotDiffOnceAsync(CancellationToken cancellationToken)
    {
        return RefreshSnapshotAsync(cancellationToken);
    }

    private async Task RefreshSnapshotAsync(CancellationToken cancellationToken)
    {
        HostSnapshot? snapshotToPublish = null;

        await _gate.WaitAsync(cancellationToken);

        try
        {
            snapshotToPublish = UpdateSnapshotCore(emitIfChanged: true);
        }
        finally
        {
            _gate.Release();
        }

        if (snapshotToPublish is not null)
        {
            SnapshotChanged?.Invoke(this, snapshotToPublish);
        }
    }

    private HostSnapshot? UpdateSnapshotCore(bool emitIfChanged)
    {
        var effectiveStatus = GetEffectiveStatusCore();
        var displays = effectiveStatus == Device.Status.OK
            ? Core.GetDisplays()
            : new List<Display>();

        var config = _configurationService.GetSnapshot();
        var candidate = ContractMapper.BuildSnapshot(
            _snapshot.Revision + 1,
            effectiveStatus,
            _driverVersion,
            displays,
            config);

        var fingerprint = HostSnapshotFingerprint.Compute(candidate);
        if (_snapshot.Revision > 0 && string.Equals(_snapshotFingerprint, fingerprint, StringComparison.Ordinal))
        {
            return null;
        }

        _snapshot = candidate;
        _snapshotFingerprint = fingerprint;

        return emitIfChanged ? candidate : null;
    }

    private void EnsureDriverReadyCore(bool requireHandle)
    {
        RefreshDriverStateCore();

        if (_status != Device.Status.OK)
        {
            throw new ErrorDriverStatus(ContractMapper.ToDriverState(_status));
        }

        if (requireHandle && !_handle.IsValidHandle())
        {
            throw new ErrorDeviceHandle();
        }
    }

    private Display? ResolveDisplay(IReadOnlyList<Display> displays, int? index)
    {
        if (index is null)
        {
            return displays.LastOrDefault();
        }

        var display = displays.FirstOrDefault(candidate => candidate.DisplayIndex == index.Value);
        return display ?? throw new ErrorDisplayNotFound(index.Value);
    }

    private void RefreshDriverStateCore()
    {
        var status = Core.QueryStatus(out var driverVersion);

        if (status != Device.Status.OK)
        {
            CloseHandleCore();
            _status = status;
            _driverVersion = driverVersion;
            return;
        }

        if (!_handle.IsValidHandle())
        {
            if (!Core.OpenHandle(out var handle))
            {
                _status = Device.Status.INACCESSIBLE;
                _driverVersion = driverVersion;
                return;
            }

            _handle = handle;
        }

        _status = Device.Status.OK;
        _driverVersion = driverVersion;
    }

    private Device.Status GetEffectiveStatusCore()
    {
        return _status == Device.Status.OK && !_handle.IsValidHandle()
            ? Device.Status.INACCESSIBLE
            : _status;
    }

    private void CloseHandleCore()
    {
        if (_handle.IsValidHandle())
        {
            Device.CloseHandle(_handle);
            _handle = IntPtr.Zero;
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }
}
