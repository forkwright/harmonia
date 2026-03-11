// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.RootFolders;

namespace Mouseion.Core.HealthCheck.Checks;

public class RootFolderCheck : IProvideHealthCheck
{
    private readonly IRootFolderService _rootFolderService;

    public RootFolderCheck(IRootFolderService rootFolderService)
    {
        _rootFolderService = rootFolderService;
    }

    public HealthCheck Check()
    {
        var rootFolders = _rootFolderService.GetAll();

        if (rootFolders.Count == 0)
        {
            return new HealthCheck(
                HealthCheckResult.Warning,
                "No root folders configured",
                "root-folder-missing"
            );
        }

        var inaccessibleFolders = rootFolders
            .Where(rf => !Directory.Exists(rf.Path))
            .Select(rf => rf.Path)
            .ToList();

        if (inaccessibleFolders.Count > 0)
        {
            return new HealthCheck(
                HealthCheckResult.Error,
                $"Root folders are inaccessible: {string.Join(", ", inaccessibleFolders)}",
                "root-folder-inaccessible"
            );
        }

        return new HealthCheck(
            HealthCheckResult.Ok,
            "All root folders are accessible"
        );
    }
}
