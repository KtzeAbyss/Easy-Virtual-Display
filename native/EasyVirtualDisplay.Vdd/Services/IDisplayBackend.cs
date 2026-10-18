using System;
using System.Threading;
using System.Threading.Tasks;
using EasyVirtualDisplay.Vdd.Domain;

namespace EasyVirtualDisplay.Vdd.Services;

/// <summary>The display operations <see cref="KeyedDisplays"/> builds on; implemented by <see cref="VddSession"/>.</summary>
public interface IDisplayBackend
{
    event EventHandler<HostSnapshot>? SnapshotChanged;

    Task<HostSnapshot> GetSnapshotAsync(CancellationToken cancellationToken = default);

    Task<int> AddDisplayAsync(CancellationToken cancellationToken = default);

    Task RemoveDisplayAsync(int? index = null, CancellationToken cancellationToken = default);
}
