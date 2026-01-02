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
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common
{
    public interface IConsoleService
    {
        void PrintHelp();
        void PrintServiceAlreadyExist();
        void PrintServiceDoesNotExist();
    }

    public class ConsoleService : IConsoleService
    {
        public const string SERVICE_NAME = "Mouseion";

        public static bool IsConsoleAvailable => Console.In != StreamReader.Null;

        public void PrintHelp()
        {
            Console.WriteLine();
            Console.WriteLine("     Usage: {0} <command> ", Process.GetCurrentProcess().MainModule?.ModuleName);
            Console.WriteLine("     Commands:");

            if (OsInfo.IsWindows)
            {
                Console.WriteLine("                 /{0} Install the application as a Windows Service ({1}).", StartupContext.INSTALL_SERVICE, SERVICE_NAME);
                Console.WriteLine("                 /{0} Uninstall already installed Windows Service ({1}).", StartupContext.UNINSTALL_SERVICE, SERVICE_NAME);
                Console.WriteLine("                 /{0} Register URL and open firewall port (allows access from other devices on your network).", StartupContext.REGISTER_URL);
            }

            Console.WriteLine("                 /{0} Don't open Mouseion in a browser", StartupContext.NO_BROWSER);
            Console.WriteLine("                 /{0} Start Mouseion terminating any other instances", StartupContext.TERMINATE);
            Console.WriteLine("                 /{0}=path Path to use as the AppData location (stores database, config, logs, etc)", StartupContext.APPDATA);
            Console.WriteLine("                 <No Arguments>  Run application in console mode.");
        }

        public void PrintServiceAlreadyExist()
        {
            Console.WriteLine("A service with the same name ({0}) already exists. Aborting installation", SERVICE_NAME);
        }

        public void PrintServiceDoesNotExist()
        {
            Console.WriteLine("Can't find service ({0})", SERVICE_NAME);
        }
    }
}
