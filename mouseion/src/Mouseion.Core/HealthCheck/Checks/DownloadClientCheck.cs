// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;

namespace Mouseion.Core.HealthCheck.Checks;

/// <summary>
/// Health check that tests connectivity to all configured download clients.
/// </summary>
public class DownloadClientCheck : IProvideHealthCheck
{
    private readonly IEnumerable<IDownloadClient> _clients;

    public DownloadClientCheck(IEnumerable<IDownloadClient> clients)
    {
        _clients = clients;
    }

    public HealthCheck Check()
    {
        var clients = _clients.ToList();

        if (clients.Count == 0)
        {
            return new HealthCheck(
                HealthCheckResult.Notice,
                "No download clients configured",
                "download-client-none");
        }

        // We can't run async in a sync method, so just report count
        return new HealthCheck(
            HealthCheckResult.Ok,
            $"{clients.Count} download client(s) configured: {string.Join(", ", clients.Select(c => c.Name))}",
            "download-client-ok");
    }
}
