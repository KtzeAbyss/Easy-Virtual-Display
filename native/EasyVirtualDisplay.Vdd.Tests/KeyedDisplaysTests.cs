using EasyVirtualDisplay.Vdd.Domain;
using EasyVirtualDisplay.Vdd.Services;

namespace EasyVirtualDisplay.Vdd.Tests;

public class KeyedDisplaysTests
{
    /// <summary>In-memory driver: added displays show up (inactive, then active) on demand.</summary>
    /// <remarks>Tests drive it while <see cref="KeyedDisplays"/> polls from the thread pool, hence the lock.</remarks>
    private sealed class FakeBackend : IDisplayBackend
    {
        private readonly object _lock = new();
        private readonly List<DisplaySummary> _displays = new();
        private readonly HashSet<int> _pending = new();
        private long _revision;

        public event EventHandler<HostSnapshot>? SnapshotChanged;

        /// <summary>Whether a newly added display is enumerated right away.</summary>
        public bool AppearImmediately { get; set; } = true;

        public int AddCalls { get; private set; }

        public List<int> Removed { get; } = new();

        public Task<HostSnapshot> GetSnapshotAsync(CancellationToken cancellationToken = default)
        {
            return Task.FromResult(Capture());
        }

        public Task<int> AddDisplayAsync(CancellationToken cancellationToken = default)
        {
            int index;

            lock (_lock)
            {
                AddCalls++;
                index = Enumerable.Range(0, 8).First(i => !_pending.Contains(i) && _displays.All(d => d.Index != i));
                _pending.Add(index);
            }

            if (AppearImmediately)
            {
                Show(index, active: true);
            }

            return Task.FromResult(index);
        }

        public Task RemoveDisplayAsync(int? index = null, CancellationToken cancellationToken = default)
        {
            lock (_lock)
            {
                var display = _displays.FirstOrDefault(d => d.Index == index)
                    ?? throw new InvalidOperationException($"Display {index} not found.");
                Removed.Add(display.Index);
            }

            RemoveBehindTheBack(index!.Value);
            return Task.CompletedTask;
        }

        /// <summary>
        /// The display gets enumerated (or changes state). Without <paramref name="publish"/>, the
        /// snapshot showing it is taken but its event has not arrived yet.
        /// </summary>
        public void Show(int index, bool active, bool publish = true)
        {
            HostSnapshot snapshot;

            lock (_lock)
            {
                _pending.Remove(index);
                _displays.RemoveAll(d => d.Index == index);
                _displays.Add(new DisplaySummary { Index = index, Active = active });
                snapshot = SnapshotCore();
            }

            if (publish)
            {
                Deliver(snapshot);
            }
        }

        /// <summary>
        /// The display goes away without the keyed API, e.g. removed from the UI. Without
        /// <paramref name="publish"/>, the snapshot is taken but its event has not arrived yet.
        /// </summary>
        public HostSnapshot RemoveBehindTheBack(int index, bool publish = true)
        {
            HostSnapshot snapshot;

            lock (_lock)
            {
                _displays.RemoveAll(d => d.Index == index);
                snapshot = SnapshotCore();
            }

            if (publish)
            {
                Deliver(snapshot);
            }

            return snapshot;
        }

        /// <summary>Takes a snapshot of the current displays without publishing it.</summary>
        public HostSnapshot Capture()
        {
            lock (_lock)
            {
                return SnapshotCore();
            }
        }

        /// <summary>Publishes <paramref name="snapshot"/>, which may be older than one already published.</summary>
        public void Deliver(HostSnapshot snapshot) => SnapshotChanged?.Invoke(this, snapshot);

        private HostSnapshot SnapshotCore() => new()
        {
            Revision = ++_revision,
            Status = DriverStatuses.Ok,
            Displays = _displays.ToArray(),
        };
    }

    private static KeyedDisplays Create(FakeBackend backend, TimeSpan? appearTimeout = null) =>
        new(backend, TimeSpan.FromMilliseconds(5), appearTimeout ?? TimeSpan.FromSeconds(10));

    [Fact]
    public async Task Ensure_AddsOnceAndReusesTheDisplayForTheSameKey()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);

        var first = await keyed.EnsureAsync("sunshine", wait: null);
        var second = await keyed.EnsureAsync("sunshine", wait: null);

        Assert.Equal(first, second);
        Assert.Equal(1, backend.AddCalls);
    }

    [Fact]
    public async Task Ensure_GivesEachKeyItsOwnDisplay()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);

        var a = await keyed.EnsureAsync("a", wait: null);
        var b = await keyed.EnsureAsync("b", wait: null);

        Assert.NotEqual(a, b);
        Assert.Equal(2, backend.AddCalls);
    }

    [Fact]
    public async Task Ensure_DoesNotAddAgainWhileTheNewDisplayIsNotEnumeratedYet()
    {
        var backend = new FakeBackend { AppearImmediately = false };
        using var keyed = Create(backend);

        await keyed.EnsureAsync("k", wait: null);
        await keyed.EnsureAsync("k", wait: null);

        Assert.Equal(1, backend.AddCalls);
    }

    [Fact]
    public async Task Ensure_AddsAgainWhenTheDisplayNeverShowsUp()
    {
        var backend = new FakeBackend { AppearImmediately = false };
        using var keyed = Create(backend, appearTimeout: TimeSpan.FromMilliseconds(20));

        var first = await keyed.EnsureAsync("k", wait: null);
        await Task.Delay(50);
        var second = await keyed.EnsureAsync("k", wait: null);

        Assert.NotEqual(first, second);
        Assert.Equal(2, backend.AddCalls);
        Assert.Equal(new Dictionary<string, int> { ["k"] = second }, keyed.GetBindings());
    }

    [Fact]
    public async Task Ensure_AddsAgainAfterTheDisplayWasRemovedElsewhere()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);

        var index = await keyed.EnsureAsync("k", wait: null);
        backend.RemoveBehindTheBack(index);
        await keyed.EnsureAsync("k", wait: null);

        Assert.Equal(2, backend.AddCalls);
    }

    [Fact]
    public async Task Ensure_KeepsTheBindingWhenAnOlderSnapshotArrivesLate()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);
        var stale = backend.Capture();

        var index = await keyed.EnsureAsync("k", wait: null);
        backend.Deliver(stale);

        Assert.Equal(new Dictionary<string, int> { ["k"] = index }, keyed.GetBindings());
    }

    [Fact]
    public async Task Ensure_KeepsTheBindingWhenAnOlderSnapshotArrivesAfterThePollSawTheDisplay()
    {
        var backend = new FakeBackend { AppearImmediately = false };
        using var keyed = Create(backend);

        var index = await keyed.EnsureAsync("k", wait: null);
        var stale = backend.Capture();
        backend.Show(index, active: true, publish: false);
        await keyed.EnsureAsync("k", wait: TimeSpan.FromSeconds(5));
        backend.Deliver(stale);

        Assert.Equal(new Dictionary<string, int> { ["k"] = index }, keyed.GetBindings());
        Assert.Equal(1, backend.AddCalls);
    }

    [Fact]
    public async Task Release_StillWaitsWhenAnOlderSnapshotShowsThePreviousDisplayAtTheIndex()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);
        var previous = await backend.AddDisplayAsync();
        var stale = backend.Capture();
        backend.RemoveBehindTheBack(previous);

        backend.AppearImmediately = false;
        var index = await keyed.EnsureAsync("k", wait: null);
        Assert.Equal(previous, index);
        backend.Deliver(stale);

        var release = keyed.ReleaseAsync("k");
        await Task.Delay(30);
        backend.Show(index, active: true);

        Assert.True(await release);
        Assert.Equal(new[] { index }, backend.Removed);
    }

    [Fact]
    public async Task Release_SparesTheUsersDisplayWhenItShowsUpBeforeTheRemovalOfTheKeyedOne()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);

        var index = await keyed.EnsureAsync("k", wait: null);
        var removal = backend.RemoveBehindTheBack(index, publish: false);
        Assert.Equal(index, await backend.AddDisplayAsync()); // the user's display takes the index
        backend.Deliver(removal);

        Assert.False(await keyed.ReleaseAsync("k"));
        Assert.Empty(backend.Removed);
    }

    [Fact]
    public async Task Ensure_WaitsUntilTheDisplayIsActive()
    {
        var backend = new FakeBackend { AppearImmediately = false };
        using var keyed = Create(backend);

        var ensure = keyed.EnsureAsync("k", wait: TimeSpan.FromSeconds(5));
        await Task.Delay(30);
        Assert.False(ensure.IsCompleted);

        backend.Show(0, active: false);
        await Task.Delay(30);
        Assert.False(ensure.IsCompleted);

        backend.Show(0, active: true);
        Assert.Equal(0, await ensure);
    }

    [Fact]
    public async Task Ensure_TimesOutWhenTheDisplayNeverBecomesActive()
    {
        var backend = new FakeBackend { AppearImmediately = false };
        using var keyed = Create(backend);

        await Assert.ThrowsAsync<TimeoutException>(
            () => keyed.EnsureAsync("k", wait: TimeSpan.FromMilliseconds(50)));
        Assert.Equal(new Dictionary<string, int> { ["k"] = 0 }, keyed.GetBindings());
    }

    [Fact]
    public async Task Release_RemovesOnlyTheKeyedDisplay()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);
        await backend.AddDisplayAsync(); // the user's own display, index 0

        var index = await keyed.EnsureAsync("k", wait: null);
        Assert.True(await keyed.ReleaseAsync("k"));

        Assert.Equal(new[] { index }, backend.Removed);
        Assert.Empty(keyed.GetBindings());
    }

    [Fact]
    public async Task Release_OfAnUnknownKeyChangesNothing()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);
        await backend.AddDisplayAsync();

        Assert.False(await keyed.ReleaseAsync("nobody"));
        Assert.Empty(backend.Removed);
    }

    [Fact]
    public async Task Release_DoesNotTouchADisplayThatReusedTheIndex()
    {
        var backend = new FakeBackend();
        using var keyed = Create(backend);

        var index = await keyed.EnsureAsync("k", wait: null);
        backend.RemoveBehindTheBack(index);
        await backend.AddDisplayAsync(); // the user adds a display that takes the same index

        Assert.False(await keyed.ReleaseAsync("k"));
        Assert.Empty(backend.Removed);
    }

    [Fact]
    public async Task Release_WaitsForAJustAddedDisplayBeforeRemovingIt()
    {
        var backend = new FakeBackend { AppearImmediately = false };
        using var keyed = Create(backend);

        var index = await keyed.EnsureAsync("k", wait: null);
        var release = keyed.ReleaseAsync("k");
        await Task.Delay(30);
        backend.Show(index, active: true);

        Assert.True(await release);
        Assert.Equal(new[] { index }, backend.Removed);
    }
}
