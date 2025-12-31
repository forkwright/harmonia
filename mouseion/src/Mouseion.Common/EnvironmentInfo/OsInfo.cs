// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Serilog;

namespace Mouseion.Common.EnvironmentInfo
{
    public class OsInfo : IOsInfo
    {
        public static Os Os { get; }

        public static bool IsNotWindows => !IsWindows;
        public static bool IsLinux => Os == Os.Linux || Os == Os.LinuxMusl || Os == Os.Bsd;
        public static bool IsOsx => Os == Os.Osx;
        public static bool IsWindows => Os == Os.Windows;

        public bool IsDocker { get; }

        public string Version { get; }
        public string Name { get; }
        public string FullName { get; }

        static OsInfo()
        {
            if (OperatingSystem.IsWindows())
            {
                Os = Os.Windows;
            }
            else if (OperatingSystem.IsMacOS())
            {
                Os = Os.Osx;
            }
            else if (OperatingSystem.IsFreeBSD())
            {
                Os = Os.Bsd;
            }
            else
            {
#if ISMUSL
                Os = Os.LinuxMusl;
#else
                Os = Os.Linux;
#endif
            }
        }

        public OsInfo(IEnumerable<IOsVersionAdapter> versionAdapters, ILogger logger)
        {
            OsVersionModel osInfo = null;

            foreach (var osVersionAdapter in versionAdapters.Where(c => c.Enabled))
            {
                try
                {
                    osInfo = osVersionAdapter.Read();
                }
                catch (Exception e)
                {
                    logger.Error(e, "Couldn't get OS Version info");
                }

                if (osInfo != null)
                {
                    break;
                }
            }

            if (osInfo != null)
            {
                Name = osInfo.Name;
                Version = osInfo.Version;
                FullName = osInfo.FullName;
            }
            else
            {
                Name = Os.ToString();
                FullName = Name;
            }

            if (IsLinux &&
                (File.Exists("/.dockerenv") ||
                 (File.Exists("/proc/1/cgroup") && File.ReadAllText("/proc/1/cgroup").Contains("/docker/"))))
            {
                IsDocker = true;
            }
        }
    }

    public interface IOsInfo
    {
        string Version { get; }
        string Name { get; }
        string FullName { get; }
        bool IsDocker { get; }
    }

    public enum Os
    {
        Windows,
        Linux,
        Osx,
        LinuxMusl,
        Bsd
    }
}
