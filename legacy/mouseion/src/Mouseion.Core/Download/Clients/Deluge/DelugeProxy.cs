// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.Deluge;

/// <summary>
/// Low-level Deluge Web UI JSON-RPC proxy.
/// Auth: web.login(password) returns a session cookie.
/// Torrent listing: core.get_torrents_status with field filter.
/// </summary>
public class DelugeProxy
{
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<DelugeProxy> _logger;
    private HttpClient? _httpClient;
    private CookieContainer? _cookies;
    private bool _authenticated;
    private int _requestId;

    private static readonly string[] TorrentFields =
    {
        "hash", "name", "state", "total_size", "progress", "eta",
        "ratio", "save_path", "label", "message", "is_finished",
        "total_remaining", "paused", "tracker_status"
    };

    public DelugeProxy(IHttpClientFactory httpClientFactory, ILogger<DelugeProxy> logger)
    {
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    private string BuildUrl(DelugeSettings settings)
    {
        var protocol = settings.UseSsl ? "https" : "http";
        var urlBase = string.IsNullOrWhiteSpace(settings.UrlBase)
            ? string.Empty
            : $"/{settings.UrlBase.Trim('/')}";

        return $"{protocol}://{settings.Host}:{settings.Port}{urlBase}/json";
    }

    private HttpClient GetClient()
    {
        if (_httpClient == null)
        {
            _cookies = new CookieContainer();
            var handler = new HttpClientHandler
            {
                CookieContainer = _cookies,
                UseCookies = true
            };
            _httpClient = new HttpClient(handler);
        }
        return _httpClient;
    }

    private async Task<DelugeJsonRpcResponse<T>> ExecuteRpcAsync<T>(
        DelugeSettings settings,
        string method,
        object[]? parameters = null,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings);
        var client = GetClient();

        var request = new DelugeJsonRpcRequest
        {
            Method = method,
            Params = parameters ?? Array.Empty<object>(),
            Id = ++_requestId
        };

        var json = JsonSerializer.Serialize(request);
        using var content = new StringContent(json, Encoding.UTF8, "application/json");
        using var response = await client.PostAsync(url, content, cancellationToken);
        response.EnsureSuccessStatusCode();

        var responseText = await response.Content.ReadAsStringAsync(cancellationToken);
        return JsonSerializer.Deserialize<DelugeJsonRpcResponse<T>>(responseText)
            ?? throw new InvalidOperationException("Failed to deserialize Deluge RPC response");
    }

    private async Task AuthenticateAsync(DelugeSettings settings, CancellationToken cancellationToken)
    {
        if (_authenticated)
        {
            return;
        }

        var result = await ExecuteRpcAsync<bool>(settings, "auth.login",
            new object[] { settings.Password }, cancellationToken);

        if (result.Result != true)
        {
            throw new HttpRequestException("Deluge authentication failed — invalid password");
        }

        // Check connection to daemon
        var connected = await ExecuteRpcAsync<bool>(settings, "web.connected",
            cancellationToken: cancellationToken);

        if (connected.Result != true)
        {
            // Try to connect to first available host
            var hosts = await ExecuteRpcAsync<List<List<JsonElement>>>(settings, "web.get_hosts",
                cancellationToken: cancellationToken);

            if (hosts.Result != null && hosts.Result.Count > 0)
            {
                var hostId = hosts.Result[0][0].GetString();
                if (hostId != null)
                {
                    await ExecuteRpcAsync<object>(settings, "web.connect",
                        new object[] { hostId }, cancellationToken);
                }
            }
        }

        _authenticated = true;
        _logger.LogDebug("Authenticated with Deluge Web UI");
    }

    public async Task<Dictionary<string, DelugeTorrent>> GetTorrentsAsync(
        DelugeSettings settings,
        CancellationToken cancellationToken = default)
    {
        await AuthenticateAsync(settings, cancellationToken);

        var result = await ExecuteRpcAsync<Dictionary<string, JsonElement>>(
            settings, "core.get_torrents_status",
            new object[] { new { }, TorrentFields },
            cancellationToken);

        if (result.Error != null)
        {
            throw new HttpRequestException($"Deluge RPC error: {result.Error.Message}");
        }

        var torrents = new Dictionary<string, DelugeTorrent>();

        if (result.Result != null)
        {
            foreach (var (hash, element) in result.Result)
            {
                var torrent = JsonSerializer.Deserialize<DelugeTorrent>(element.GetRawText());
                if (torrent != null)
                {
                    torrent.Hash = hash;
                    torrents[hash] = torrent;
                }
            }
        }

        return torrents;
    }

    public async Task<string> GetVersionAsync(
        DelugeSettings settings,
        CancellationToken cancellationToken = default)
    {
        await AuthenticateAsync(settings, cancellationToken);

        var result = await ExecuteRpcAsync<string>(settings, "daemon.info",
            cancellationToken: cancellationToken);

        return result.Result ?? "unknown";
    }

    public async Task RemoveTorrentAsync(
        string hash,
        bool deleteData,
        DelugeSettings settings,
        CancellationToken cancellationToken = default)
    {
        await AuthenticateAsync(settings, cancellationToken);

        await ExecuteRpcAsync<bool>(settings, "core.remove_torrent",
            new object[] { hash, deleteData }, cancellationToken);
    }

    public async Task<bool> TestConnectionAsync(
        DelugeSettings settings,
        CancellationToken cancellationToken = default)
    {
        try
        {
            _authenticated = false;
            await AuthenticateAsync(settings, cancellationToken);
            var version = await GetVersionAsync(settings, cancellationToken);
            _logger.LogDebug("Connected to Deluge {Version}", version);
            return true;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error testing Deluge connection");
            return false;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse Deluge response during connection test");
            return false;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out testing Deluge connection");
            return false;
        }
    }
}
