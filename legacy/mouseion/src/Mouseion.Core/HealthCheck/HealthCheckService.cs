// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.HealthCheck;

public interface IHealthCheckService
{
    List<HealthCheck> PerformHealthChecks();
    HealthCheck? GetHealthCheck(string source);
}

public class HealthCheckService : IHealthCheckService
{
    private readonly IEnumerable<IProvideHealthCheck> _healthCheckProviders;
    private readonly ILogger<HealthCheckService> _logger;
    private readonly ConcurrentDictionary<string, HealthCheck> _healthCheckResults = new();

    public HealthCheckService(
        IEnumerable<IProvideHealthCheck> healthCheckProviders,
        ILogger<HealthCheckService> logger)
    {
        _healthCheckProviders = healthCheckProviders;
        _logger = logger;
    }

    public List<HealthCheck> PerformHealthChecks()
    {
        _logger.LogDebug("Performing health checks");
        var results = new List<HealthCheck>();

        foreach (var provider in _healthCheckProviders)
        {
            try
            {
                var result = provider.Check();
                var key = provider.GetType().Name;
                _healthCheckResults[key] = result;
                results.Add(result);
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Health check failed for {Provider} (invalid operation)", provider.GetType().Name);
                var errorCheck = new HealthCheck(
                    HealthCheckResult.Error,
                    $"Health check failed: {ex.Message}"
                );
                results.Add(errorCheck);
            }
            catch (IOException ex)
            {
                _logger.LogError(ex, "Health check failed for {Provider} (I/O error)", provider.GetType().Name);
                var errorCheck = new HealthCheck(
                    HealthCheckResult.Error,
                    $"Health check failed: {ex.Message}"
                );
                results.Add(errorCheck);
            }
            catch (HttpRequestException ex)
            {
                _logger.LogError(ex, "Health check failed for {Provider} (network error)", provider.GetType().Name);
                var errorCheck = new HealthCheck(
                    HealthCheckResult.Error,
                    $"Health check failed: {ex.Message}"
                );
                results.Add(errorCheck);
            }
        }

        _logger.LogDebug("Completed {Count} health checks", results.Count);
        return results;
    }

    public HealthCheck? GetHealthCheck(string source)
    {
        _healthCheckResults.TryGetValue(source, out var result);
        return result;
    }
}
