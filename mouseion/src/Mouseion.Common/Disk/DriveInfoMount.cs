// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;
using Mouseion.Common.Extensions;

namespace Mouseion.Common.Disk
{
    public class DriveInfoMount : IMount
    {
        private readonly DriveInfo _driveInfo;
        private readonly DriveType _driveType;

        public DriveInfoMount(DriveInfo driveInfo, DriveType driveType = DriveType.Unknown, MountOptions? mountOptions = null)
        {
            _driveInfo = driveInfo;
            _driveType = driveType;
            MountOptions = mountOptions;
        }

        public long AvailableFreeSpace => _driveInfo.AvailableFreeSpace;

        public string DriveFormat => _driveInfo.DriveFormat;

        public DriveType DriveType
        {
            get
            {
                if (_driveType != DriveType.Unknown)
                {
                    return _driveType;
                }

                return _driveInfo.DriveType;
            }
        }

        public bool IsReady => _driveInfo.IsReady;

        public MountOptions? MountOptions { get; private set; }

        public string Name => _driveInfo.Name;

        public string RootDirectory => _driveInfo.RootDirectory.FullName;

        public long TotalFreeSpace => _driveInfo.TotalFreeSpace;

        public long TotalSize => _driveInfo.TotalSize;

        public string VolumeLabel => _driveInfo.VolumeLabel;

        public string VolumeName
        {
            get
            {
                if (VolumeLabel.IsNullOrWhiteSpace() || VolumeLabel.StartsWith("UUID=") || Name == VolumeLabel)
                {
                    return Name;
                }

                return $"{Name} ({VolumeLabel})";
            }
        }
    }
}
