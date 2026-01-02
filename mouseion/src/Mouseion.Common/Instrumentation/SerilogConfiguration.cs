// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.IO;
using Mouseion.Common.EnvironmentInfo;
using Serilog;
using Serilog.Events;

namespace Mouseion.Common.Instrumentation
{
    public static class SerilogConfiguration
    {
        private static bool _initialized;
        private static readonly object _lock = new object();

        public static void Initialize(IAppFolderInfo appFolderInfo, LogEventLevel minimumLevel = LogEventLevel.Information)
        {
            lock (_lock)
            {
                if (_initialized)
                {
                    return;
                }

                var logFolder = Path.Combine(appFolderInfo.AppDataFolder, "logs");
                Directory.CreateDirectory(logFolder);
                var logPath = Path.Combine(logFolder, "mouseion.txt");

                var configuration = new LoggerConfiguration()
                    .MinimumLevel.Is(minimumLevel)
                    .MinimumLevel.Override("Microsoft", LogEventLevel.Warning)
                    .MinimumLevel.Override("System", LogEventLevel.Warning)
                    .Enrich.FromLogContext()
                    .Enrich.WithProperty("Application", BuildInfo.AppName)
                    .Enrich.WithProperty("Version", BuildInfo.Version)
                    .WriteTo.Console(
                        outputTemplate: "[{Timestamp:HH:mm:ss} {Level:u3}] {Message:lj}{NewLine}{Exception}")
                    .WriteTo.File(
                        logPath,
                        rollingInterval: RollingInterval.Day,
                        retainedFileCountLimit: 7,
                        outputTemplate: "{Timestamp:yyyy-MM-dd HH:mm:ss.fff zzz} [{Level:u3}] {Message:lj}{NewLine}{Exception}");

                Log.Logger = configuration.CreateLogger();
                _initialized = true;
            }
        }

        public static void InitializeConsoleOnly(LogEventLevel minimumLevel = LogEventLevel.Debug)
        {
            lock (_lock)
            {
                if (_initialized)
                {
                    return;
                }

                var configuration = new LoggerConfiguration()
                    .MinimumLevel.Is(minimumLevel)
                    .Enrich.FromLogContext()
                    .WriteTo.Console(
                        outputTemplate: "[{Timestamp:HH:mm:ss} {Level:u3}] {Message:lj}{NewLine}{Exception}");

                Log.Logger = configuration.CreateLogger();
                _initialized = true;
            }
        }

        public static void SetMinimumLevel(LogEventLevel level)
        {
            // Note: Serilog doesn't support dynamic level changes with the basic configuration.
            // For dynamic level changes, use Serilog.Settings.Configuration or implement a LoggingLevelSwitch.
            Log.Logger.Warning("Dynamic log level changes require reconfiguration. Current level change to {Level} noted.", level);
        }

        public static ILogger GetLogger(Type type)
        {
            return Log.ForContext(type);
        }

        public static ILogger GetLogger<T>()
        {
            return Log.ForContext<T>();
        }

        public static void CloseAndFlush()
        {
            Log.CloseAndFlush();
        }
    }
}
