// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Polly;
using Polly.CircuitBreaker;
using Polly.Retry;

namespace Mouseion.Core.MetadataSource;

/// <summary>
/// HTTP client wrapper with Polly resilience (circuit breaker + retry)
/// </summary>
public class ResilientMetadataClient
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<ResilientMetadataClient> _logger;
    private readonly ResiliencePipeline _pipeline;

    public ResilientMetadataClient(IHttpClient httpClient, ILogger<ResilientMetadataClient> _logger)
    {
        _httpClient = httpClient;
        this._logger = _logger;

        _pipeline = new ResiliencePipelineBuilder()
            .AddRetry(new RetryStrategyOptions
            {
                MaxRetryAttempts = 3,
                Delay = TimeSpan.FromSeconds(1),
                BackoffType = DelayBackoffType.Exponential,
                ShouldHandle = new PredicateBuilder().Handle<HttpRequestException>(),
                OnRetry = args =>
                {
                    _logger.LogWarning("Metadata API retry attempt {AttemptNumber} after {Delay}ms",
                        args.AttemptNumber, args.RetryDelay.TotalMilliseconds);
                    return ValueTask.CompletedTask;
                }
            })
            .AddCircuitBreaker(new CircuitBreakerStrategyOptions
            {
                FailureRatio = 0.5,
                SamplingDuration = TimeSpan.FromSeconds(30),
                MinimumThroughput = 5,
                BreakDuration = TimeSpan.FromSeconds(30),
                ShouldHandle = new PredicateBuilder().Handle<HttpRequestException>(),
                OnOpened = args =>
                {
                    _logger.LogError("Metadata API circuit breaker opened - API calls suspended for {BreakDuration}",
                        args.BreakDuration);
                    return ValueTask.CompletedTask;
                },
                OnClosed = args =>
                {
                    _logger.LogInformation("Metadata API circuit breaker closed - API calls resumed");
                    return ValueTask.CompletedTask;
                },
                OnHalfOpened = args =>
                {
                    _logger.LogInformation("Metadata API circuit breaker half-open - testing connectivity");
                    return ValueTask.CompletedTask;
                }
            })
            .Build();
    }

    public HttpResponse Get(HttpRequest request)
    {
        try
        {
            return _pipeline.Execute(() => _httpClient.GetAsync(request).GetAwaiter().GetResult());
        }
        catch (BrokenCircuitException ex)
        {
            _logger.LogWarning(ex, "Metadata API circuit breaker is open - request rejected");
            throw;
        }
    }

    public async Task<HttpResponse> GetAsync(HttpRequest request)
    {
        try
        {
            return await _pipeline.ExecuteAsync(async ct => await _httpClient.GetAsync(request), CancellationToken.None);
        }
        catch (BrokenCircuitException ex)
        {
            _logger.LogWarning(ex, "Metadata API circuit breaker is open - request rejected");
            throw;
        }
    }
}
