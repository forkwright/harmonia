// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Generic;

namespace Mouseion.Common.Disk
{
    public class FileSystemResult
    {
        public string? Parent { get; set; }
        public List<FileSystemModel> Directories { get; set; }
        public List<FileSystemModel> Files { get; set; }

        public FileSystemResult()
        {
            Directories = new List<FileSystemModel>();
            Files = new List<FileSystemModel>();
        }
    }
}
