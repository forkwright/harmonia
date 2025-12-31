// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Disk
{
    public class FileAlreadyExistsException : Exception
    {
        public string Filename { get; set; }

        public FileAlreadyExistsException(string message, string filename)
            : base(message)
        {
            Filename = filename;
        }
    }
}
