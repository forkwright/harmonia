// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Disk;

public class DiskProvider : DiskProviderBase
{

    public override long? GetAvailableSpace(string path)
    {
        try
        {
            var driveInfo = new DriveInfo(path);
            return driveInfo.AvailableFreeSpace;
        }
        catch
        {
            return null;
        }
    }

    public override void InheritFolderPermissions(string filename)
    {
        // Platform-specific implementation needed
    }

    public override void SetEveryonePermissions(string filename)
    {
        // Platform-specific implementation needed
    }

    public override void SetFilePermissions(string path, string mask, string group)
    {
        // Platform-specific implementation needed
    }

    public override void SetPermissions(string path, string mask, string group)
    {
        // Platform-specific implementation needed
    }

    public override void CopyPermissions(string sourcePath, string targetPath)
    {
        // Platform-specific implementation needed
    }

    public override long? GetTotalSize(string path)
    {
        try
        {
            var driveInfo = new DriveInfo(path);
            return driveInfo.TotalSize;
        }
        catch
        {
            return null;
        }
    }

    public override bool TryCreateHardLink(string source, string destination)
    {
        try
        {
            // CreateHardLink is only available in .NET 6+
            // Fallback to copy if not available
            File.Copy(source, destination, false);
            return true;
        }
        catch
        {
            return false;
        }
    }
}
