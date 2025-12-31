// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Linq;
using System.Runtime.Versioning;
using System.ServiceProcess;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Extensions;
using Mouseion.Common.Processes;
using Serilog;

namespace Mouseion.Common
{
    public interface IServiceProvider
    {
        bool ServiceExist(string name);
        bool IsServiceRunning(string name);
        void Install(string serviceName);
        void Uninstall(string serviceName);
        void Run(ServiceBase service);
        ServiceController? GetService(string serviceName);
        void Stop(string serviceName);
        void Start(string serviceName);
        ServiceControllerStatus GetStatus(string serviceName);
        void Restart(string serviceName);
        void SetPermissions(string serviceName);
    }

    public class ServiceProvider : IServiceProvider
    {
        public const string SERVICE_NAME = "Mouseion";

        private readonly IProcessProvider _processProvider;
        private readonly ILogger _logger;

        public ServiceProvider(IProcessProvider processProvider, ILogger logger)
        {
            _processProvider = processProvider;
            _logger = logger;
        }

        public virtual bool ServiceExist(string name)
        {
            _logger.Debug("Checking if service {Name} exists.", name);

            if (!OsInfo.IsWindows)
            {
                return false;
            }

#pragma warning disable CA1416
            return ServiceController.GetServices().Any(
                s => string.Equals(s.ServiceName, name, StringComparison.InvariantCultureIgnoreCase));
#pragma warning restore CA1416
        }

        public virtual bool IsServiceRunning(string name)
        {
            _logger.Debug("Checking if '{Name}' service is running", name);

            if (!OsInfo.IsWindows)
            {
                return false;
            }

#pragma warning disable CA1416
            var service = ServiceController.GetServices()
                .SingleOrDefault(s => string.Equals(s.ServiceName, name, StringComparison.InvariantCultureIgnoreCase));

            return service != null && (
                service.Status != ServiceControllerStatus.Stopped ||
                service.Status == ServiceControllerStatus.StopPending ||
                service.Status == ServiceControllerStatus.Paused ||
                service.Status == ServiceControllerStatus.PausePending);
#pragma warning restore CA1416
        }

        public virtual void Install(string serviceName)
        {
            _logger.Information("Installing service '{ServiceName}'", serviceName);

            var args = $"create {serviceName} " +
                $"DisplayName= \"{serviceName}\" " +
                $"binpath= \"{Environment.ProcessPath}\" " +
                "start= auto " +
                "depend= EventLog/Tcpip/http " +
                "obj= \"NT AUTHORITY\\LocalService\"";

            _logger.Information("{Args}", args);

            var installOutput = _processProvider.StartAndCapture("sc.exe", args);

            if (installOutput.ExitCode != 0)
            {
                _logger.Error("Failed to install service: {Output}", installOutput.Lines.Select(x => x.Content).ConcatToString("\n"));
                throw new ServiceInstallationException("Failed to install service");
            }

            _logger.Information("{Output}", installOutput.Lines.Select(x => x.Content).ConcatToString("\n"));

            var descOutput = _processProvider.StartAndCapture("sc.exe", $"description {serviceName} \"Mouseion Application Server\"");
            if (descOutput.ExitCode != 0)
            {
                _logger.Error("Failed to install service: {Output}", descOutput.Lines.Select(x => x.Content).ConcatToString("\n"));
                throw new ServiceInstallationException("Failed to install service");
            }

            _logger.Information("{Output}", descOutput.Lines.Select(x => x.Content).ConcatToString("\n"));

            _logger.Information("Service Has installed successfully.");
        }

        public virtual void Uninstall(string serviceName)
        {
            _logger.Information("Uninstalling {ServiceName} service", serviceName);

            Stop(serviceName);

            var output = _processProvider.StartAndCapture("sc.exe", $"delete {serviceName}");
            _logger.Information("{Output}", output.Lines.Select(x => x.Content).ConcatToString("\n"));

            _logger.Information("{ServiceName} successfully uninstalled", serviceName);
        }

        public virtual void Run(ServiceBase service)
        {
            if (!OsInfo.IsWindows)
            {
                throw new PlatformNotSupportedException("Windows services are only supported on Windows");
            }

#pragma warning disable CA1416
            ServiceBase.Run(service);
#pragma warning restore CA1416
        }

        public virtual ServiceController? GetService(string serviceName)
        {
            if (!OsInfo.IsWindows)
            {
                return null;
            }

#pragma warning disable CA1416
            return ServiceController.GetServices().FirstOrDefault(c => string.Equals(c.ServiceName, serviceName, StringComparison.InvariantCultureIgnoreCase));
#pragma warning restore CA1416
        }

        public virtual void Stop(string serviceName)
        {
            if (!OsInfo.IsWindows)
            {
                _logger.Warning("Service operations are only supported on Windows");
                return;
            }

#pragma warning disable CA1416
            _logger.Information("Stopping {ServiceName} Service...", serviceName);
            var service = GetService(serviceName);
            if (service == null)
            {
                _logger.Warning("Unable to stop {ServiceName}. no service with that name exists.", serviceName);
                return;
            }

            _logger.Information("Service is currently {Status}", service.Status);

            if (service.Status != ServiceControllerStatus.Stopped)
            {
                service.Stop();
                service.WaitForStatus(ServiceControllerStatus.Stopped, TimeSpan.FromSeconds(60));

                service.Refresh();
                if (service.Status == ServiceControllerStatus.Stopped)
                {
                    _logger.Information("{ServiceName} has stopped successfully.", serviceName);
                }
                else
                {
                    _logger.Error("Service stop request has timed out. {Status}", service.Status);
                }
            }
            else
            {
                _logger.Warning("Service {ServiceName} is already in stopped state.", service.ServiceName);
            }
#pragma warning restore CA1416
        }

        public ServiceControllerStatus GetStatus(string serviceName)
        {
            if (!OsInfo.IsWindows)
            {
                return ServiceControllerStatus.Stopped;
            }

#pragma warning disable CA1416
            return GetService(serviceName)!.Status;
#pragma warning restore CA1416
        }

        public void Start(string serviceName)
        {
            if (!OsInfo.IsWindows)
            {
                _logger.Warning("Service operations are only supported on Windows");
                return;
            }

#pragma warning disable CA1416
            _logger.Information("Starting {ServiceName} Service...", serviceName);
            var service = GetService(serviceName);
            if (service == null)
            {
                _logger.Warning("Unable to start '{ServiceName}' no service with that name exists.", serviceName);
                return;
            }

            if (service.Status != ServiceControllerStatus.Paused && service.Status != ServiceControllerStatus.Stopped)
            {
                _logger.Warning("Service is in a state that can't be started. Current status: {Status}", service.Status);
            }

            service.Start();

            service.WaitForStatus(ServiceControllerStatus.Running, TimeSpan.FromSeconds(60));
            service.Refresh();

            if (service.Status == ServiceControllerStatus.Running)
            {
                _logger.Information("{ServiceName} has started successfully.", serviceName);
            }
            else
            {
                _logger.Error("Service start request has timed out. {Status}", service.Status);
            }
#pragma warning restore CA1416
        }

        public void Restart(string serviceName)
        {
            _logger.Information("Restarting {ServiceName} Service...", serviceName);
            Stop(serviceName);
            Start(serviceName);
        }

        public void SetPermissions(string serviceName)
        {
            var dacls = GetServiceDacls(serviceName);
            SetServiceDacls(serviceName, dacls);
        }

        private string GetServiceDacls(string serviceName)
        {
            var output = _processProvider.StartAndCapture("sc.exe", $"sdshow {serviceName}");

            var dacls = output.Standard.Select(s => s.Content).Where(s => s.IsNotNullOrWhiteSpace()).ToList();

            if (dacls.Count == 1)
            {
                return dacls[0];
            }

            throw new ArgumentException("Invalid DACL output");
        }

        private void SetServiceDacls(string serviceName, string dacls)
        {
            const string authenticatedUsersDacl = "(A;;CCLCSWRPWPLOCRRC;;;AU)";

            if (dacls.Contains(authenticatedUsersDacl))
            {
                return;
            }

            var indexOfS = dacls.IndexOf("S:", StringComparison.InvariantCultureIgnoreCase);

            dacls = indexOfS == -1 ? $"{dacls}{authenticatedUsersDacl}" : dacls.Insert(indexOfS, authenticatedUsersDacl);

            _processProvider.Start("sc.exe", $"sdset {serviceName} {dacls}").WaitForExit();
        }
    }
}
