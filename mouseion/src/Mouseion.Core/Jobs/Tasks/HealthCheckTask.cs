// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.HealthCheck;

namespace Mouseion.Core.Jobs.Tasks;

public class HealthCheckTask : IScheduledTask
{
    private readonly IHealthCheckService _healthCheckService;
    private readonly ILogger<HealthCheckTask> _logger;

    public HealthCheckTask(
        IHealthCheckService healthCheckService,
        ILogger<HealthCheckTask> logger)
    {
        _healthCheckService = healthCheckService;
        _logger = logger;
    }

    public string Name => "HealthCheck";
    public TimeSpan Interval => TimeSpan.FromHours(6);

    public Task ExecuteAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Running health checks");
        var results = _healthCheckService.PerformHealthChecks();

        var errors = results.Count(r => r.Type == HealthCheckResult.Error);
        var warnings = results.Count(r => r.Type == HealthCheckResult.Warning);

        if (errors > 0 || warnings > 0)
        {
            _logger.LogWarning("Health check completed: {Errors} errors, {Warnings} warnings", errors, warnings);
        }
        else
        {
            _logger.LogInformation("Health check completed: All systems healthy");
        }

        return Task.CompletedTask;
    }
}
