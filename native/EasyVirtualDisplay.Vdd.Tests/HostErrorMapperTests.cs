using System.Text.Json;
using EasyVirtualDisplay.Host.Errors;
using EasyVirtualDisplay.Vdd.Domain;

namespace EasyVirtualDisplay.Vdd.Tests;

public class HostErrorMapperTests
{
    private static JsonElement ParseMessage(Exception ex)
    {
        var json = HostErrorMapper.ToMessage(ex);
        return JsonSerializer.Deserialize<JsonElement>(json);
    }

    private static string GetCode(JsonElement el) =>
        el.GetProperty("code").GetString()!;

    [Fact]
    public void NotInstalled_MapsToDriverNotInstalled()
    {
        var ex = new ErrorDriverStatus(DriverState.NotInstalled);
        Assert.Equal("driver_not_installed", GetCode(ParseMessage(ex)));
    }

    [Fact]
    public void Disabled_MapsToDriverDisabled()
    {
        var ex = new ErrorDriverStatus(DriverState.Disabled);
        Assert.Equal("driver_disabled", GetCode(ParseMessage(ex)));
    }

    [Fact]
    public void DisabledService_MapsToDriverDisabled()
    {
        var ex = new ErrorDriverStatus(DriverState.DisabledService);
        Assert.Equal("driver_disabled", GetCode(ParseMessage(ex)));
    }

    [Fact]
    public void RestartRequired_MapsToDriverRestartRequired()
    {
        var ex = new ErrorDriverStatus(DriverState.RestartRequired);
        Assert.Equal("driver_restart_required", GetCode(ParseMessage(ex)));
    }

    [Fact]
    public void LimitExceeded_MapsToLimitExceeded()
    {
        var ex = new ErrorExceededLimit(4);
        var el = ParseMessage(ex);
        Assert.Equal("limit_exceeded", GetCode(el));
        Assert.Equal(4, el.GetProperty("details").GetProperty("limit").GetInt32());
    }

    [Fact]
    public void DisplayNotFound_MapsToDisplayNotFound()
    {
        var ex = new ErrorDisplayNotFound(2);
        var el = ParseMessage(ex);
        Assert.Equal("display_not_found", GetCode(el));
        Assert.Equal(2, el.GetProperty("details").GetProperty("index").GetInt32());
    }

    [Fact]
    public void UnsupportedMode_MapsToUnsupportedMode()
    {
        var ex = new ErrorUnsupportedMode(0, 1920, 1080, 60, null);
        var el = ParseMessage(ex);
        Assert.Equal("unsupported_mode", GetCode(el));
    }

    [Fact]
    public void ArgumentException_MapsToDriverError()
    {
        var ex = new ArgumentException("bad arg");
        Assert.Equal("driver_error", GetCode(ParseMessage(ex)));
    }

    [Fact]
    public void JsonException_MapsToDriverError()
    {
        var ex = new JsonException("bad json");
        Assert.Equal("driver_error", GetCode(ParseMessage(ex)));
    }

    [Fact]
    public void UnknownException_MapsToDriverError()
    {
        var ex = new InvalidOperationException("something failed");
        Assert.Equal("driver_error", GetCode(ParseMessage(ex)));
    }
}
