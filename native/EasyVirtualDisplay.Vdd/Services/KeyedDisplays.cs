using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using EasyVirtualDisplay.Vdd.Domain;

namespace EasyVirtualDisplay.Vdd.Services;

/// <summary>
/// Displays owned by a caller-chosen key (say, a streaming session), so that a release
/// removes exactly the display the matching ensure added and nothing the user added.
/// </summary>
/// <remarks>
/// Bindings live as long as the host process: quitting the app removes every display
/// anyway. A new display takes a few seconds to be enumerated, so a binding only drops
/// out on its own (the display was removed elsewhere) after its display has been seen.
/// </remarks>
public sealed class KeyedDisplays : IDisposable
{
    private static readonly TimeSpan DefaultPollInterval = TimeSpan.FromMilliseconds(250);
    private static readonly TimeSpan DefaultAppearTimeout = TimeSpan.FromSeconds(10);

    private readonly IDisplayBackend _backend;
    private readonly TimeSpan _pollInterval;
    private readonly TimeSpan _appearTimeout;
    private readonly SemaphoreSlim _gate = new(1, 1);
    private readonly object _bindingsLock = new();
    private readonly Dictionary<string, Binding> _bindings = new(StringComparer.Ordinal);

    public KeyedDisplays(IDisplayBackend backend)
        : this(backend, DefaultPollInterval, DefaultAppearTimeout)
    {
    }

    internal KeyedDisplays(IDisplayBackend backend, TimeSpan pollInterval, TimeSpan appearTimeout)
    {
        _backend = backend;
        _pollInterval = pollInterval;
        _appearTimeout = appearTimeout;
        _backend.SnapshotChanged += HandleSnapshotChanged;
    }

    /// <summary>
    /// Returns the index of the display bound to <paramref name="key"/>, adding one when
    /// there is none. With <paramref name="wait"/>, also waits until that display is active.
    /// </summary>
    /// <exception cref="TimeoutException">The display was not active within <paramref name="wait"/>.</exception>
    public async Task<int> EnsureAsync(string key, TimeSpan? wait, CancellationToken cancellationToken = default)
    {
        int index;

        await _gate.WaitAsync(cancellationToken);

        try
        {
            var snapshot = await _backend.GetSnapshotAsync(cancellationToken);
            var existing = Find(key);

            // A display not enumerated within the appear timeout is taken to never show up (say,
            // the driver restarted first): add another rather than return its index for good.
            // The driver cannot be asked whether it still holds that display, so one that shows
            // up later is left over (visible, and removed when the app quits) rather than risk this.
            // Until then a display the user adds at that index would pass for it; that needs a
            // failed ensure followed by a manual add, so it is left alone.
            if (existing is not null
                && (existing.Seen
                    ? HasDisplay(snapshot, existing.Index)
                    : DateTime.UtcNow - existing.AddedAt < _appearTimeout))
            {
                index = existing.Index;
            }
            else
            {
                index = await _backend.AddDisplayAsync(cancellationToken);

                // The backend publishes before returning, so that snapshot never reached the binding.
                var added = await _backend.GetSnapshotAsync(cancellationToken);
                var binding = new Binding(index, added.Revision);
                if (HasDisplay(added, index))
                {
                    binding.MarkSeen(added.Revision);
                }

                lock (_bindingsLock)
                {
                    // A binding to the same index belongs to a display that is already gone.
                    foreach (var stale in _bindings.Where(pair => pair.Value.Index == index).Select(pair => pair.Key).ToList())
                    {
                        _bindings.Remove(stale);
                    }

                    _bindings[key] = binding;
                }
            }
        }
        finally
        {
            _gate.Release();
        }

        if (wait is not null)
        {
            await WaitForAsync(index, display => display.Active, wait.Value, cancellationToken);
        }

        return index;
    }

    /// <summary>Removes the display bound to <paramref name="key"/>. Returns false when no display is bound to it.</summary>
    public async Task<bool> ReleaseAsync(string key, CancellationToken cancellationToken = default)
    {
        await _gate.WaitAsync(cancellationToken);

        try
        {
            var binding = Find(key);
            if (binding is null)
            {
                return false;
            }

            if (!binding.Seen)
            {
                // Added moments ago: it can only be removed once it is enumerated.
                try
                {
                    await WaitForAsync(binding.Index, _ => true, _appearTimeout, cancellationToken);
                }
                catch (TimeoutException)
                {
                    // Drop the binding anyway: if the display never shows up, keeping it would make
                    // every later ensure return that phantom index and every release time out again.
                    // One that shows up later is left over, as in EnsureAsync.
                }
            }

            // Not serialized with display changes from the UI: the removal queues on the session
            // lock right after this check, so the UI would have to remove this display and add
            // another at its index in between, two clicks within the moment that takes.
            var snapshot = await _backend.GetSnapshotAsync(cancellationToken);
            if (HasDisplay(snapshot, binding.Index))
            {
                await _backend.RemoveDisplayAsync(binding.Index, cancellationToken);
            }

            lock (_bindingsLock)
            {
                _bindings.Remove(key);
            }

            return true;
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>Display index bound to each key.</summary>
    public IReadOnlyDictionary<string, int> GetBindings()
    {
        lock (_bindingsLock)
        {
            return _bindings.ToDictionary(pair => pair.Key, pair => pair.Value.Index, StringComparer.Ordinal);
        }
    }

    public void Dispose()
    {
        _backend.SnapshotChanged -= HandleSnapshotChanged;
        _gate.Dispose();
    }

    private Binding? Find(string key)
    {
        lock (_bindingsLock)
        {
            return _bindings.TryGetValue(key, out var binding) ? binding : null;
        }
    }

    private async Task WaitForAsync(
        int index,
        Func<DisplaySummary, bool> ready,
        TimeSpan timeout,
        CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow + timeout;

        // Goes by index alone. For another display to take the index meanwhile, someone has to
        // remove this one and add another within the second or two it takes to activate, and
        // that display is then rightly not released with this key.
        while (true)
        {
            var snapshot = await _backend.GetSnapshotAsync(cancellationToken);
            var display = snapshot.Displays.FirstOrDefault(candidate => candidate.Index == index);
            if (display is not null)
            {
                MarkSeen(index, snapshot.Revision);
                if (ready(display))
                {
                    return;
                }
            }

            if (DateTime.UtcNow >= deadline)
            {
                throw new TimeoutException($"Display {index} was not ready within {timeout.TotalSeconds:0.#}s.");
            }

            await Task.Delay(_pollInterval, cancellationToken);
        }
    }

    private void MarkSeen(int index, long revision)
    {
        lock (_bindingsLock)
        {
            foreach (var binding in _bindings.Values.Where(binding => binding.Index == index))
            {
                binding.MarkSeen(revision);
            }
        }
    }

    private void HandleSnapshotChanged(object? sender, HostSnapshot snapshot)
    {
        lock (_bindingsLock)
        {
            foreach (var (key, binding) in _bindings.ToList())
            {
                if (HasDisplay(snapshot, binding.Index))
                {
                    binding.MarkSeen(snapshot.Revision);
                }
                // Snapshots are published outside the session lock, so they can arrive out of
                // order: one from before the display first showed up says nothing, while one
                // after it means the display went away, even if a newer one showing another
                // display at the same index arrived first.
                else if (binding.Seen && snapshot.Revision > binding.SeenAt)
                {
                    _bindings.Remove(key);
                }
            }
        }
    }

    private static bool HasDisplay(HostSnapshot snapshot, int index)
    {
        return snapshot.Displays.Any(display => display.Index == index);
    }

    private sealed class Binding(int index, long since)
    {
        public int Index { get; } = index;

        /// <summary>
        /// Revision of the snapshot read right after adding. An older one arriving late may
        /// still show the display that last held this index, so it says nothing about this one.
        /// </summary>
        public long Since { get; } = since;

        public DateTime AddedAt { get; } = DateTime.UtcNow;

        /// <summary>Revision of the earliest snapshot known to show the display; 0 until one does.</summary>
        public long SeenAt { get; private set; }

        public bool Seen => SeenAt > 0;

        public void MarkSeen(long revision)
        {
            if (revision >= Since)
            {
                SeenAt = Seen ? Math.Min(SeenAt, revision) : revision;
            }
        }
    }
}
