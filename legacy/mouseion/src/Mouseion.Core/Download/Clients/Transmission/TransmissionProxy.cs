// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.Transmission;

/// <summary>
/// Low-level Transmission RPC proxy. Handles CSRF token negotiation
/// (X-Transmission-Session-Id) and basic auth.
/// </summary>
public class TransmissionProxy
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<TransmissionProxy> _logger;
    private string? _sessionId;

    private static readonly string[] TorrentFields =
    {
        "id", "hashString", "name", "status", "totalSize", "percentDone",
        "eta", "uploadRatio", "downloadDir", "labels", "errorString",
        "error", "isFinished", "leftUntilDone"
    };

    public TransmissionProxy(IHttpClientFactory httpClientFactory, ILogger<TransmissionProxy> logger)
    {
        _httpClient = httpClientFactory.CreateClient("Transmission");
        _logger = logger;
    }

    private string BuildUrl(TransmissionSettings settings)
    {
        var protocol = settings.UseSsl ? "https" : "http";
        var urlBase = string.IsNullOrWhiteSpace(settings.UrlBase)
            ? "/transmission/rpc"
            : $"/{settings.UrlBase.Trim('/')}";

        return $"{protocol}://{settings.Host}:{settings.Port}{urlBase}";
    }

    private async Task<TransmissionResponse<T>> ExecuteRpcAsync<T>(
        TransmissionSettings settings,
        string method,
        object? arguments = null,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings);
        var request = new TransmissionRequest { Method = method, Arguments = arguments };
        var json = JsonSerializer.Serialize(request);

        for (var attempt = 0; attempt < 2; attempt++)
        {
            using var httpRequest = new HttpRequestMessage(HttpMethod.Post, url);
            httpRequest.Content = new StringContent(json, Encoding.UTF8, "application/json");

            if (!string.IsNullOrEmpty(settings.Username))
            {
                var credentials = Convert.ToBase64String(
                    Encoding.UTF8.GetBytes($"{settings.Username}:{settings.Password}"));
                httpRequest.Headers.Authorization = new AuthenticationHeaderValue("Basic", credentials);
            }

            if (!string.IsNullOrEmpty(_sessionId))
            {
                httpRequest.Headers.Add("X-Transmission-Session-Id", _sessionId);
            }

            using var response = await _httpClient.SendAsync(httpRequest, cancellationToken);

            // Transmission returns 409 with session ID header on first request
            if (response.StatusCode == HttpStatusCode.Conflict)
            {
                if (response.Headers.TryGetValues("X-Transmission-Session-Id", out var values))
                {
                    _sessionId = values.FirstOrDefault();
                    _logger.LogDebug("Acquired Transmission session ID");
                    continue; // Retry with session ID
                }

                throw new HttpRequestException("Transmission returned 409 without session ID header");
            }

            response.EnsureSuccessStatusCode();
            var responseText = await response.Content.ReadAsStringAsync(cancellationToken);
            return JsonSerializer.Deserialize<TransmissionResponse<T>>(responseText)
                ?? throw new InvalidOperationException("Failed to deserialize Transmission RPC response");
        }

        throw new HttpRequestException("Failed to authenticate with Transmission after 2 attempts");
    }

    public async Task<List<TransmissionTorrent>> GetTorrentsAsync(
        TransmissionSettings settings,
        CancellationToken cancellationToken = default)
    {
        var response = await ExecuteRpcAsync<TransmissionTorrentList>(
            settings, "torrent-get",
            new { fields = TorrentFields },
            cancellationToken);

        if (!response.IsSuccess)
        {
            throw new HttpRequestException($"Transmission RPC error: {response.Result}");
        }

        return response.Arguments?.Torrents ?? new List<TransmissionTorrent>();
    }

    public async Task<TransmissionSessionInfo> GetSessionInfoAsync(
        TransmissionSettings settings,
        CancellationToken cancellationToken = default)
    {
        var response = await ExecuteRpcAsync<TransmissionSessionInfo>(
            settings, "session-get", cancellationToken: cancellationToken);

        if (!response.IsSuccess)
        {
            throw new HttpRequestException($"Transmission RPC error: {response.Result}");
        }

        return response.Arguments
            ?? throw new InvalidOperationException("Transmission returned null session info");
    }

    public async Task RemoveTorrentAsync(
        string hashString,
        bool deleteData,
        TransmissionSettings settings,
        CancellationToken cancellationToken = default)
    {
        // Find torrent ID by hash
        var torrents = await GetTorrentsAsync(settings, cancellationToken);
        var torrent = torrents.FirstOrDefault(t =>
            t.HashString.Equals(hashString, StringComparison.OrdinalIgnoreCase));

        if (torrent == null)
        {
            _logger.LogWarning("Torrent {Hash} not found in Transmission", hashString);
            return;
        }

        var response = await ExecuteRpcAsync<object>(
            settings, "torrent-remove",
            new { ids = new[] { torrent.Id }, delete_local_data = deleteData },
            cancellationToken);

        if (!response.IsSuccess)
        {
            throw new HttpRequestException($"Failed to remove torrent: {response.Result}");
        }
    }

    public async Task<bool> TestConnectionAsync(
        TransmissionSettings settings,
        CancellationToken cancellationToken = default)
    {
        try
        {
            await GetSessionInfoAsync(settings, cancellationToken);
            return true;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error testing Transmission connection");
            return false;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse Transmission response during connection test");
            return false;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out testing Transmission connection");
            return false;
        }
    }
}
