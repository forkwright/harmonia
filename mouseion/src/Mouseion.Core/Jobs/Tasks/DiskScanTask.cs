// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.RootFolders;

namespace Mouseion.Core.Jobs.Tasks;

public class DiskScanTask : IScheduledTask
{
    private readonly IRootFolderService _rootFolderService;
    private readonly ILogger<DiskScanTask> _logger;

    public DiskScanTask(
        IRootFolderService rootFolderService,
        ILogger<DiskScanTask> logger)
    {
        _rootFolderService = rootFolderService;
        _logger = logger;
    }

    public string Name => "DiskScan";
    public TimeSpan Interval => TimeSpan.FromHours(24);

    public Task ExecuteAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Running disk scan");
        var rootFolders = _rootFolderService.GetAll();

        foreach (var rootFolder in rootFolders)
        {
            if (!Directory.Exists(rootFolder.Path))
            {
                _logger.LogWarning("Root folder does not exist: {Path}", rootFolder.Path);
                continue;
            }

            try
            {
                var driveInfo = new DriveInfo(Path.GetPathRoot(rootFolder.Path)!);
                _logger.LogInformation(
                    "Disk space for {Path}: {Free:N0} GB free of {Total:N0} GB total",
                    rootFolder.Path,
                    driveInfo.AvailableFreeSpace / 1024.0 / 1024.0 / 1024.0,
                    driveInfo.TotalSize / 1024.0 / 1024.0 / 1024.0
                );
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to get disk info for {Path}", rootFolder.Path);
            }
        }

        _logger.LogInformation("Disk scan completed");
        return Task.CompletedTask;
    }
}
