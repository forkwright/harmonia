// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Diagnostics;
using System.IO;
using System.Security.Principal;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Hosting.WindowsServices;
using Serilog;

namespace Mouseion.Common.EnvironmentInfo
{
    public class RuntimeInfo : IRuntimeInfo
    {
        public const string MOUSEION_PROCESS_NAME = "Mouseion";

        private readonly ILogger _logger;
        private readonly DateTime _startTime = DateTime.UtcNow;

        public RuntimeInfo(ILogger logger, IHostLifetime hostLifetime = null)
        {
            _logger = logger;

            IsWindowsService = hostLifetime is WindowsServiceLifetime;
            IsStarting = true;

            var entry = Process.GetCurrentProcess().MainModule;

            if (entry != null)
            {
                ExecutingApplication = entry.FileName;
                IsWindowsTray = OsInfo.IsWindows && entry.ModuleName == $"{MOUSEION_PROCESS_NAME}.exe";
            }
        }

        static RuntimeInfo()
        {
            var officialBuild = InternalIsOfficialBuild();

            IsTesting = InternalIsTesting();
            IsProduction = !IsTesting && officialBuild;
            IsDevelopment = !IsTesting && !officialBuild && !InternalIsDebug();
        }

        public DateTime StartTime
        {
            get
            {
                return _startTime;
            }
        }

        public static bool IsUserInteractive => Environment.UserInteractive;

        bool IRuntimeInfo.IsUserInteractive => IsUserInteractive;

        public bool IsAdmin
        {
            get
            {
                if (OsInfo.IsNotWindows)
                {
                    return false;
                }

                try
                {
                    var principal = new WindowsPrincipal(WindowsIdentity.GetCurrent());
                    return principal.IsInRole(WindowsBuiltInRole.Administrator);
                }
                catch (Exception ex)
                {
                    _logger.Warning(ex, "Error checking if the current user is an administrator.");
                    return false;
                }
            }
        }

        public bool IsWindowsService { get; private set; }

        public bool IsStarting { get; set; }
        public bool IsExiting { get; set; }
        public bool IsTray
        {
            get
            {
                if (OsInfo.IsWindows)
                {
                    return IsUserInteractive && Process.GetCurrentProcess().ProcessName.Equals(MOUSEION_PROCESS_NAME, StringComparison.InvariantCultureIgnoreCase);
                }

                return false;
            }
        }

        public RuntimeMode Mode
        {
            get
            {
                if (IsWindowsService)
                {
                    return RuntimeMode.Service;
                }

                if (IsTray)
                {
                    return RuntimeMode.Tray;
                }

                return RuntimeMode.Console;
            }
        }

        public bool RestartPending { get; set; }
        public string ExecutingApplication { get; }

        public static bool IsTesting { get; }
        public static bool IsProduction { get; }
        public static bool IsDevelopment { get; }

        private static bool InternalIsTesting()
        {
            try
            {
                var lowerProcessName = Process.GetCurrentProcess().ProcessName.ToLower();

                if (lowerProcessName.Contains("vshost"))
                {
                    return true;
                }

                if (lowerProcessName.Contains("nunit"))
                {
                    return true;
                }

                if (lowerProcessName.Contains("jetbrain"))
                {
                    return true;
                }

                if (lowerProcessName.Contains("resharper"))
                {
                    return true;
                }
            }
            catch
            {
            }

            try
            {
                var currentAssemblyLocation = typeof(RuntimeInfo).Assembly.Location;
                if (currentAssemblyLocation.ToLower().Contains("_output"))
                {
                    return true;
                }

                if (currentAssemblyLocation.ToLower().Contains("_tests"))
                {
                    return true;
                }
            }
            catch
            {
            }

            var lowerCurrentDir = Directory.GetCurrentDirectory().ToLower();
            if (lowerCurrentDir.Contains("vsts"))
            {
                return true;
            }

            if (lowerCurrentDir.Contains("buildagent"))
            {
                return true;
            }

            if (lowerCurrentDir.Contains("_output"))
            {
                return true;
            }

            if (lowerCurrentDir.Contains("_tests"))
            {
                return true;
            }

            return false;
        }

        private static bool InternalIsDebug()
        {
            if (BuildInfo.IsDebug || Debugger.IsAttached)
            {
                return true;
            }

            return false;
        }

        private static bool InternalIsOfficialBuild()
        {
            if (BuildInfo.Version.Major >= 10 || BuildInfo.Version.Revision > 20000)
            {
                return false;
            }

            return true;
        }

        public bool IsWindowsTray { get; private set; }
    }
}
