// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.IO;
using System.Reflection;
using Serilog;

namespace Mouseion.Common.EnvironmentInfo
{
    public interface IAppFolderInfo
    {
        string AppDataFolder { get; }
        string TempFolder { get; }
        string StartUpFolder { get; }
    }

    public class AppFolderInfo : IAppFolderInfo
    {
        private readonly Environment.SpecialFolder _dataSpecialFolder = Environment.SpecialFolder.CommonApplicationData;

        public AppFolderInfo(IStartupContext startupContext, ILogger logger)
        {
            if (OsInfo.IsNotWindows)
            {
                _dataSpecialFolder = Environment.SpecialFolder.ApplicationData;
            }

            if (startupContext.Args.ContainsKey(StartupContext.APPDATA))
            {
                AppDataFolder = startupContext.Args[StartupContext.APPDATA];
                logger.Information("Data directory is being overridden to [{AppDataFolder}]", AppDataFolder);
            }
            else
            {
                AppDataFolder = Path.Combine(Environment.GetFolderPath(_dataSpecialFolder, Environment.SpecialFolderOption.DoNotVerify), "Mouseion");
            }

            StartUpFolder = new FileInfo(Assembly.GetExecutingAssembly().Location).Directory.FullName;
            TempFolder = Path.GetTempPath();
        }

        public string AppDataFolder { get; private set; }

        public string StartUpFolder { get; private set; }

        public string TempFolder { get; private set; }
    }
}
