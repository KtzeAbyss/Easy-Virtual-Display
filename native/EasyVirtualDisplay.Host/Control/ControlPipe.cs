using System.IO.Pipes;
using System.Security.AccessControl;
using System.Security.Principal;

namespace EasyVirtualDisplay.Host.Control;

/// <summary>
/// The named pipe `ctl` commands reach the running host through. One request line and
/// one response line of JSON per connection.
/// </summary>
/// <remarks>
/// Access is limited to the current user by an explicit ACL rather than
/// <see cref="PipeOptions.CurrentUserOnly"/>: a pipe created by an elevated process is owned
/// by the Administrators group, which CurrentUserOnly clients reject, and the app and its
/// callers (a streaming host's prep commands, say) may each run elevated or not.
/// </remarks>
internal static class ControlPipe
{
    // Initialized before Name, which is derived from it.
    private static readonly SecurityIdentifier CurrentUser = WindowsIdentity.GetCurrent().User
        ?? throw new InvalidOperationException("The current user has no SID.");

    public static string Name { get; } = $"EasyVirtualDisplay.Control.{CurrentUser.Value}";

    public static NamedPipeServerStream CreateServer(bool firstInstance)
    {
        var security = new PipeSecurity();
        security.AddAccessRule(new PipeAccessRule(CurrentUser, PipeAccessRights.FullControl, AccessControlType.Allow));

        var options = PipeOptions.Asynchronous;
        if (firstInstance)
        {
            // Refuse to join a pipe someone else created first under our name.
            options |= PipeOptions.FirstPipeInstance;
        }

        return NamedPipeServerStreamAcl.Create(
            Name,
            PipeDirection.InOut,
            NamedPipeServerStream.MaxAllowedServerInstances,
            PipeTransmissionMode.Byte,
            options,
            inBufferSize: 0,
            outBufferSize: 0,
            security);
    }

    public static NamedPipeClientStream CreateClient()
    {
        return new NamedPipeClientStream(".", Name, PipeDirection.InOut, PipeOptions.Asynchronous);
    }
}

internal static class ControlCommands
{
    public const string Ensure = "ensure";
    public const string Release = "release";
    public const string Status = "status";
}

internal static class ControlErrorCodes
{
    public const string InvalidRequest = "invalid_request";
    public const string Timeout = "timeout";
}

internal sealed class ControlRequest
{
    public string Command { get; init; } = string.Empty;

    public string? Key { get; init; }

    /// <summary>For ensure: wait this long for the display to become active.</summary>
    public int? WaitSeconds { get; init; }
}

internal sealed class ControlResponse
{
    public bool Ok { get; init; }

    public string? Code { get; init; }

    public string? Message { get; init; }

    /// <summary>For ensure: the display bound to the key.</summary>
    public int? Index { get; init; }

    /// <summary>For release: whether a display was bound to the key.</summary>
    public bool? Released { get; init; }

    public ControlStatus? Status { get; init; }

    public static ControlResponse Failure(string code, string message) => new() { Code = code, Message = message };
}

internal sealed class ControlStatus
{
    public string DriverStatus { get; init; } = string.Empty;

    public IReadOnlyList<ControlDisplay> Displays { get; init; } = Array.Empty<ControlDisplay>();
}

internal sealed class ControlDisplay
{
    public int Index { get; init; }

    public bool Active { get; init; }

    /// <summary>The key that owns the display, or null for displays added any other way.</summary>
    public string? Key { get; init; }
}
