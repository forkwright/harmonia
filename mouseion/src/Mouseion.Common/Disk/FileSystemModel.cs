// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Disk
{
    public class FileSystemModel
    {
        public FileSystemEntityType Type { get; set; }
        public string Name { get; set; } = string.Empty;
        public string Path { get; set; } = string.Empty;
        public string Extension { get; set; } = string.Empty;
        public long Size { get; set; }
        public DateTime? LastModified { get; set; }
    }

    public enum FileSystemEntityType
    {
        Parent,
        Drive,
        Folder,
        File
    }
}
