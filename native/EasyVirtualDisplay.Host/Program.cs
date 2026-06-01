// See https://aka.ms/new-console-template for more information
using EasyVirtualDisplay.Host.Cli;
using EasyVirtualDisplay.Host.Errors;
using EasyVirtualDisplay.Host.Hosting;

try
{
    return AdminCli.CanHandle(args)
        ? await AdminCli.RunAsync(args)
        : await StdioHost.RunAsync(args);
}
catch (Exception ex)
{
    Console.Error.WriteLine(HostErrorMapper.ToMessage(ex));
    return 1;
}
