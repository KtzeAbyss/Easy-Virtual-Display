@echo off
rem Command-line control for Easy Virtual Display. Run without arguments for usage.
"%~dp0native\EasyVirtualDisplay.Host.exe" ctl %*
exit /b %ERRORLEVEL%
