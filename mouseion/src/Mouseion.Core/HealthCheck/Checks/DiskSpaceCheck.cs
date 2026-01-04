// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.RootFolders;

namespace Mouseion.Core.HealthCheck.Checks;

public class DiskSpaceCheck : IProvideHealthCheck
{
    private readonly IRootFolderService _rootFolderService;
    private const long MinimumDiskSpaceBytes = 100 * 1024 * 1024; // 100 MB

    public DiskSpaceCheck(IRootFolderService rootFolderService)
    {
        _rootFolderService = rootFolderService;
    }

    public HealthCheck Check()
    {
        var rootFolders = _rootFolderService.GetAll();

        if (rootFolders.Count == 0)
        {
            return new HealthCheck(
                HealthCheckResult.Ok,
                "No root folders to check for disk space"
            );
        }

        var lowSpaceFolders = new List<string>();

        foreach (var rootFolder in rootFolders)
        {
            if (!Directory.Exists(rootFolder.Path))
            {
                continue;
            }

            try
            {
                var driveInfo = new DriveInfo(Path.GetPathRoot(rootFolder.Path)!);
                if (driveInfo.AvailableFreeSpace < MinimumDiskSpaceBytes)
                {
                    lowSpaceFolders.Add($"{rootFolder.Path} ({FormatBytes(driveInfo.AvailableFreeSpace)} free)");
                }
            }
            catch
            {
                // Skip if unable to get drive info
                continue;
            }
        }

        if (lowSpaceFolders.Count > 0)
        {
            return new HealthCheck(
                HealthCheckResult.Warning,
                $"Low disk space on: {string.Join(", ", lowSpaceFolders)}",
                "disk-space-low"
            );
        }

        return new HealthCheck(
            HealthCheckResult.Ok,
            "Sufficient disk space available"
        );
    }

    private static string FormatBytes(long bytes)
    {
        string[] sizes = { "B", "KB", "MB", "GB", "TB" };
        double len = bytes;
        int order = 0;
        while (len >= 1024 && order < sizes.Length - 1)
        {
            order++;
            len /= 1024;
        }
        return $"{len:0.##} {sizes[order]}";
    }
}
