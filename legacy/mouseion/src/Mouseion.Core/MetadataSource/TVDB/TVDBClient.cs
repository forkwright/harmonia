// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Polly;
using Polly.Retry;

namespace Mouseion.Core.MetadataSource.TVDB;

/// <summary>
/// Low-level HTTP client for TVDB v4 API with JWT authentication and retry policies.
/// </summary>
public interface ITVDBClient
{
    Task<string?> GetAsync(string endpoint, CancellationToken ct = default);
}

public class TVDBClient : ITVDBClient
{
    private const string TokenCacheKey = "tvdb_jwt_token";
    private static readonly TimeSpan TokenCacheExpiry = TimeSpan.FromHours(23); // Tokens valid 24h, refresh early

    private readonly TVDBSettings _settings;
    private readonly IHttpClient _httpClient;
    private readonly IMemoryCache _cache;
    private readonly ILogger<TVDBClient> _logger;
    private readonly ResiliencePipeline _pipeline;
    private readonly SemaphoreSlim _tokenLock = new(1, 1);

    public TVDBClient(
        TVDBSettings settings,
        IHttpClient httpClient,
        IMemoryCache cache,
        ILogger<TVDBClient> logger)
    {
        _settings = settings;
        _httpClient = httpClient;
        _cache = cache;
        _logger = logger;

        _pipeline = new ResiliencePipelineBuilder()
            .AddRetry(new RetryStrategyOptions
            {
                MaxRetryAttempts = _settings.MaxRetries,
                Delay = TimeSpan.FromMilliseconds(500),
                BackoffType = DelayBackoffType.Exponential,
                ShouldHandle = new PredicateBuilder()
                    .Handle<HttpRequestException>()
                    .Handle<TaskCanceledException>(),
                OnRetry = args =>
                {
                    _logger.LogWarning(
                        "TVDB API retry {AttemptNumber}/{MaxAttempts} after {Delay}ms",
                        args.AttemptNumber,
                        _settings.MaxRetries,
                        args.RetryDelay.TotalMilliseconds);
                    return ValueTask.CompletedTask;
                }
            })
            .Build();
    }

    public async Task<string?> GetAsync(string endpoint, CancellationToken ct = default)
    {
        var token = await GetTokenAsync(ct).ConfigureAwait(false);
        if (string.IsNullOrEmpty(token))
        {
            _logger.LogWarning("Failed to obtain TVDB authentication token");
            return null;
        }

        var url = $"{_settings.ApiUrl.TrimEnd('/')}/{endpoint.TrimStart('/')}";

        try
        {
            return await _pipeline.ExecuteAsync(async _ =>
            {
                var request = new HttpRequestBuilder(url)
                    .SetHeader("Authorization", $"Bearer {token}")
                    .Accept(HttpAccept.Json)
                    .Build();

                var response = await _httpClient.GetAsync(request).ConfigureAwait(false);

                if (response.StatusCode == System.Net.HttpStatusCode.Unauthorized)
                {
                    _logger.LogWarning("TVDB token expired, clearing cache");
                    _cache.Remove(TokenCacheKey);
                    throw new HttpRequestException("TVDB token expired");
                }

                if (response.StatusCode != System.Net.HttpStatusCode.OK)
                {
                    _logger.LogWarning("TVDB API returned {StatusCode} for {Endpoint}", response.StatusCode, endpoint);
                    return null;
                }

                return response.Content;
            }, ct).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            _logger.LogError(ex, "Failed to fetch from TVDB API: {Endpoint}", endpoint);
            return null;
        }
    }

    private async Task<string?> GetTokenAsync(CancellationToken ct)
    {
        if (_cache.TryGetValue(TokenCacheKey, out string? cachedToken))
        {
            return cachedToken;
        }

        await _tokenLock.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            // Double-check after acquiring lock
            if (_cache.TryGetValue(TokenCacheKey, out cachedToken))
            {
                return cachedToken;
            }

            var token = await AuthenticateAsync(ct).ConfigureAwait(false);
            if (!string.IsNullOrEmpty(token))
            {
                _cache.Set(TokenCacheKey, token, TokenCacheExpiry);
            }

            return token;
        }
        finally
        {
            _tokenLock.Release();
        }
    }

    private async Task<string?> AuthenticateAsync(CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(_settings.ApiKey))
        {
            _logger.LogWarning("TVDB API key not configured");
            return null;
        }

        var loginUrl = $"{_settings.ApiUrl.TrimEnd('/')}/login";
        var payload = JsonSerializer.Serialize(new { apikey = _settings.ApiKey });

        try
        {
            _logger.LogDebug("Authenticating with TVDB API");

            var request = new HttpRequestBuilder(loginUrl)
                .Post()
                .Accept(HttpAccept.Json)
                .SetHeader("Content-Type", "application/json")
                .Build();
            request.SetContent(Encoding.UTF8.GetBytes(payload));

            var response = await _httpClient.PostAsync(request).ConfigureAwait(false);

            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogError("TVDB authentication failed with status {StatusCode}", response.StatusCode);
                return null;
            }

            using var doc = JsonDocument.Parse(response.Content);
            if (doc.RootElement.TryGetProperty("data", out var data) &&
                data.TryGetProperty("token", out var tokenElement))
            {
                var token = tokenElement.GetString();
                _logger.LogInformation("Successfully authenticated with TVDB API");
                return token;
            }

            _logger.LogError("TVDB authentication response missing token");
            return null;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to authenticate with TVDB API");
            return null;
        }
    }
}
