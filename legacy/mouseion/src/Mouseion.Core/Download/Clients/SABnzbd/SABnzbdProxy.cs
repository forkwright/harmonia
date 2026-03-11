// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.SABnzbd;

/// <summary>
/// Low-level SABnzbd API proxy. All requests authenticated via apikey query parameter.
/// </summary>
public class SABnzbdProxy
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<SABnzbdProxy> _logger;

    public SABnzbdProxy(IHttpClientFactory httpClientFactory, ILogger<SABnzbdProxy> logger)
    {
        _httpClient = httpClientFactory.CreateClient("SABnzbd");
        _logger = logger;
    }

    private string BuildUrl(SABnzbdSettings settings, string mode, string? extraParams = null)
    {
        var protocol = settings.UseSsl ? "https" : "http";
        var urlBase = string.IsNullOrWhiteSpace(settings.UrlBase)
            ? string.Empty
            : $"/{settings.UrlBase.Trim('/')}";

        var url = $"{protocol}://{settings.Host}:{settings.Port}{urlBase}/api" +
                  $"?output=json&apikey={settings.ApiKey}&mode={mode}";

        if (!string.IsNullOrEmpty(extraParams))
        {
            url += $"&{extraParams}";
        }

        return url;
    }

    public async Task<SABnzbdQueue> GetQueueAsync(
        SABnzbdSettings settings,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings, "queue");
        var response = await _httpClient.GetStringAsync(url, cancellationToken);
        var result = JsonSerializer.Deserialize<SABnzbdQueueResponse>(response);
        return result?.Queue ?? new SABnzbdQueue();
    }

    public async Task<SABnzbdHistory> GetHistoryAsync(
        SABnzbdSettings settings,
        int limit = 30,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings, "history", $"limit={limit}");
        var response = await _httpClient.GetStringAsync(url, cancellationToken);
        var result = JsonSerializer.Deserialize<SABnzbdHistoryResponse>(response);
        return result?.History ?? new SABnzbdHistory();
    }

    public async Task<SABnzbdFullStatus> GetFullStatusAsync(
        SABnzbdSettings settings,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings, "fullstatus", "skip_dashboard=1");
        var response = await _httpClient.GetStringAsync(url, cancellationToken);
        return JsonSerializer.Deserialize<SABnzbdFullStatus>(response) ?? new SABnzbdFullStatus();
    }

    public async Task<string> GetVersionAsync(
        SABnzbdSettings settings,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings, "version");
        var response = await _httpClient.GetStringAsync(url, cancellationToken);

        // SABnzbd returns {"version": "4.x.x"} in JSON mode
        using var doc = JsonDocument.Parse(response);
        return doc.RootElement.TryGetProperty("version", out var version)
            ? version.GetString() ?? "unknown"
            : "unknown";
    }

    public async Task DeleteAsync(
        string nzoId,
        bool deleteData,
        SABnzbdSettings settings,
        CancellationToken cancellationToken = default)
    {
        var mode = deleteData ? "queue" : "queue";
        var url = BuildUrl(settings, "queue",
            $"name=delete&value={nzoId}&del_files={( deleteData ? 1 : 0 )}");
        await _httpClient.GetStringAsync(url, cancellationToken);

        // Also try history delete
        var histUrl = BuildUrl(settings, "history",
            $"name=delete&value={nzoId}&del_files={( deleteData ? 1 : 0 )}");
        await _httpClient.GetStringAsync(histUrl, cancellationToken);
    }

    public async Task<bool> TestConnectionAsync(
        SABnzbdSettings settings,
        CancellationToken cancellationToken = default)
    {
        try
        {
            var version = await GetVersionAsync(settings, cancellationToken);
            _logger.LogDebug("Connected to SABnzbd {Version}", version);
            return true;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error testing SABnzbd connection");
            return false;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse SABnzbd response during connection test");
            return false;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out testing SABnzbd connection");
            return false;
        }
    }
}
