// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;

namespace Mouseion.Common.Disk
{
    public interface IMount
    {
        long AvailableFreeSpace { get; }
        string DriveFormat { get; }
        DriveType DriveType { get; }
        bool IsReady { get; }
        MountOptions? MountOptions { get; }
        string Name { get; }
        string RootDirectory { get; }
        long TotalFreeSpace { get; }
        long TotalSize { get; }
        string VolumeLabel { get; }
        string VolumeName { get; }
    }
}
