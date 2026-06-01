using System.Text.Json;
using System.Text.Json.Serialization;

namespace EasyVirtualDisplay.Host.Hosting;

internal static class HostJsonOptions
{
    public static JsonSerializerOptions Default { get; } = Create();

    private static JsonSerializerOptions Create()
    {
        return new JsonSerializerOptions(JsonSerializerDefaults.Web)
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.Never,
            WriteIndented = false,
        };
    }
}
