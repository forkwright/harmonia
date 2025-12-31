// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.IO;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Disk
{
    public static class LongPathSupport
    {
        private static int MAX_PATH;
        private static int MAX_NAME;

        public static void Enable()
        {
            AppContext.SetSwitch("Switch.System.IO.UseLegacyPathHandling", false);
            AppContext.SetSwitch("Switch.System.IO.BlockLongPaths", false);

            DetectLongPathLimits();
        }

        private static void DetectLongPathLimits()
        {
            if (!int.TryParse(Environment.GetEnvironmentVariable("MAX_PATH"), out MAX_PATH))
            {
                if (OsInfo.IsLinux)
                {
                    MAX_PATH = 4096;
                }
                else
                {
                    try
                    {
                        Path.GetDirectoryName($@"C:\{new string('a', 254)}\{new string('a', 254)}");
                        MAX_PATH = 4096;
                    }
                    catch
                    {
                        MAX_PATH = 260 - 1;
                    }
                }
            }

            if (!int.TryParse(Environment.GetEnvironmentVariable("MAX_NAME"), out MAX_NAME))
            {
                MAX_NAME = 255;
            }
        }

        public static int MaxFilePathLength
        {
            get
            {
                if (MAX_PATH == 0)
                {
                    DetectLongPathLimits();
                }

                return MAX_PATH;
            }
        }

        public static int MaxFileNameLength
        {
            get
            {
                if (MAX_NAME == 0)
                {
                    DetectLongPathLimits();
                }

                return MAX_NAME;
            }
        }
    }
}
