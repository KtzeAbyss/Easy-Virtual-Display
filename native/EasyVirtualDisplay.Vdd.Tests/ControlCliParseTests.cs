using EasyVirtualDisplay.Host.Control;

namespace EasyVirtualDisplay.Vdd.Tests;

public class ControlCliParseTests
{
    [Fact]
    public void Ensure_ParsesKeyAndDefaultWait()
    {
        var request = ControlCli.Parse(["ensure", "--key", "sunshine", "--wait"]);
        Assert.Equal(ControlCommands.Ensure, request.Command);
        Assert.Equal("sunshine", request.Key);
        Assert.Equal(15, request.WaitSeconds);
    }

    [Fact]
    public void Ensure_ParsesExplicitWaitSeconds()
    {
        var request = ControlCli.Parse(["ensure", "--key", "k", "--wait", "30"]);
        Assert.Equal(30, request.WaitSeconds);
    }

    [Fact]
    public void Release_RequiresKey()
    {
        var request = ControlCli.Parse(["release", "--key", "sunshine"]);
        Assert.Equal(ControlCommands.Release, request.Command);
        Assert.Equal("sunshine", request.Key);
        Assert.Null(request.WaitSeconds);
    }

    [Fact]
    public void Status_TakesNoKey()
    {
        var request = ControlCli.Parse(["status"]);
        Assert.Equal(ControlCommands.Status, request.Command);
        Assert.Null(request.Key);
    }

    [Fact]
    public void MissingKey_ThrowsUsageError()
    {
        var ex = Assert.Throws<ArgumentException>(() => ControlCli.Parse(["ensure"]));
        Assert.Contains("--key", ex.Message);
    }

    [Fact]
    public void UnknownCommand_ThrowsUsageError()
    {
        Assert.Throws<ArgumentException>(() => ControlCli.Parse(["nope"]));
    }
}
