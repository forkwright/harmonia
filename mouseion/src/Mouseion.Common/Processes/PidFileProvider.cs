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
using Mouseion.Common.Exceptions;
using Serilog;

namespace Mouseion.Common.Processes
{
    public interface IProvidePidFile
    {
        void Write();
    }

    public class PidFileProvider : IProvidePidFile
    {
        private readonly IAppFolderInfo _appFolderInfo;
        private readonly ILogger _logger;

        public PidFileProvider(IAppFolderInfo appFolderInfo, ILogger logger)
        {
            _appFolderInfo = appFolderInfo;
            _logger = logger;
        }

        public void Write()
        {
            if (OsInfo.IsWindows)
            {
                return;
            }

            var filename = Path.Combine(_appFolderInfo.AppDataFolder, "mouseion.pid");
            try
            {
                File.WriteAllText(filename, ProcessProvider.GetCurrentProcessId().ToString());
            }
            catch (IOException ex)
            {
                _logger.Error(ex, "Unable to write PID file: {Filename} (I/O error)", filename);
                throw new MouseionStartupException(ex, "Unable to write PID file {0}", filename);
            }
            catch (UnauthorizedAccessException ex)
            {
                _logger.Error(ex, "Unable to write PID file: {Filename} (access denied)", filename);
                throw new MouseionStartupException(ex, "Unable to write PID file {0}", filename);
            }
        }
    }
}
