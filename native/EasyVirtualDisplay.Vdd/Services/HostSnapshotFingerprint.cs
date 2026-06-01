using System.Text.Json;
using EasyVirtualDisplay.Vdd.Domain;

namespace EasyVirtualDisplay.Vdd.Services;

internal static class HostSnapshotFingerprint
{
    private static readonly JsonSerializerOptions SerializerOptions = new(JsonSerializerDefaults.Web);

    public static string Compute(HostSnapshot snapshot)
    {
        return JsonSerializer.Serialize(
            new
            {
                snapshot.Status,
                snapshot.DriverVersion,
                snapshot.MaxDisplays,
                snapshot.Displays,
                snapshot.CustomModes,
                snapshot.ParentGpu,
            },
            SerializerOptions);
    }
}
