// See https://aka.ms/new-console-template for more information
using EasyVirtualDisplay.Host.Cli;
using EasyVirtualDisplay.Host.Control;
using EasyVirtualDisplay.Host.Errors;
using EasyVirtualDisplay.Host.Hosting;

try
{
    if (ControlCli.CanHandle(args))
    {
        return await ControlCli.RunAsync(args);
    }

    return AdminCli.CanHandle(args)
        ? await AdminCli.RunAsync(args)
        : await StdioHost.RunAsync(args);
}
catch (Exception ex)
{
    Console.Error.WriteLine(HostErrorMapper.ToMessage(ex));
    return 1;
}
