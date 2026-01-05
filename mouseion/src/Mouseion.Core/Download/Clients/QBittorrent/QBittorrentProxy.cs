// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.QBittorrent;

public class QBittorrentProxy
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<QBittorrentProxy> _logger;
    private string? _authCookie;

    public QBittorrentProxy(IHttpClientFactory httpClientFactory, ILogger<QBittorrentProxy> logger)
    {
        _httpClient = httpClientFactory.CreateClient("QBittorrent");
        _logger = logger;
    }

    private static string BuildUrl(QBittorrentSettings settings, string resource)
    {
        var protocol = settings.UseSsl ? "https" : "http";
        var urlBase = string.IsNullOrWhiteSpace(settings.UrlBase)
            ? string.Empty
            : $"/{settings.UrlBase.Trim('/')}";

        return $"{protocol}://{settings.Host}:{settings.Port}{urlBase}{resource}";
    }

    private async Task AuthenticateAsync(QBittorrentSettings settings, CancellationToken cancellationToken)
    {
        if (!string.IsNullOrEmpty(_authCookie))
        {
            return; // Already authenticated
        }

        var url = BuildUrl(settings, "/api/v2/auth/login");
        var content = new FormUrlEncodedContent(new Dictionary<string, string>
        {
            ["username"] = settings.Username,
            ["password"] = settings.Password
        });

        using var request = new HttpRequestMessage(HttpMethod.Post, url) { Content = content };
        using var response = await _httpClient.SendAsync(request, cancellationToken);

        if (!response.IsSuccessStatusCode)
        {
            throw new HttpRequestException($"Authentication failed: {response.StatusCode}");
        }

        // Extract authentication cookie
        if (response.Headers.TryGetValues("Set-Cookie", out var cookies))
        {
            _authCookie = cookies.FirstOrDefault();
            _logger.LogDebug("QBittorrent authentication successful");
        }
        else
        {
            var responseText = await response.Content.ReadAsStringAsync(cancellationToken);
            if (responseText != "Ok.")
            {
                throw new HttpRequestException("Authentication failed - invalid credentials");
            }
        }
    }

    private async Task<string> ExecuteRequestAsync(
        QBittorrentSettings settings,
        string resource,
        HttpMethod? method = null,
        HttpContent? content = null,
        CancellationToken cancellationToken = default)
    {
        await AuthenticateAsync(settings, cancellationToken);

        var url = BuildUrl(settings, resource);
        using var request = new HttpRequestMessage(method ?? HttpMethod.Get, url);

        if (_authCookie != null)
        {
            request.Headers.Add("Cookie", _authCookie);
        }

        if (content != null)
        {
            request.Content = content;
        }

        using var response = await _httpClient.SendAsync(request, cancellationToken);
        response.EnsureSuccessStatusCode();

        return await response.Content.ReadAsStringAsync(cancellationToken);
    }

    private async Task<T> ExecuteRequestAsync<T>(
        QBittorrentSettings settings,
        string resource,
        CancellationToken cancellationToken = default)
    {
        var responseText = await ExecuteRequestAsync(settings, resource, cancellationToken: cancellationToken);
        return JsonSerializer.Deserialize<T>(responseText)
            ?? throw new InvalidOperationException("Failed to deserialize response");
    }

    public async Task<Version> GetApiVersionAsync(QBittorrentSettings settings, CancellationToken cancellationToken = default)
    {
        var versionString = await ExecuteRequestAsync(settings, "/api/v2/app/webapiVersion", cancellationToken: cancellationToken);
        return Version.Parse(versionString.Trim('"'));
    }

    public async Task<QBittorrentPreferences> GetConfigAsync(QBittorrentSettings settings, CancellationToken cancellationToken = default)
    {
        return await ExecuteRequestAsync<QBittorrentPreferences>(settings, "/api/v2/app/preferences", cancellationToken);
    }

    public async Task<List<QBittorrentTorrent>> GetTorrentsAsync(QBittorrentSettings settings, CancellationToken cancellationToken = default)
    {
        var resource = "/api/v2/torrents/info";
        if (!string.IsNullOrWhiteSpace(settings.Category))
        {
            resource += $"?category={Uri.EscapeDataString(settings.Category)}";
        }

        return await ExecuteRequestAsync<List<QBittorrentTorrent>>(settings, resource, cancellationToken);
    }

    public async Task<Dictionary<string, QBittorrentLabel>> GetLabelsAsync(QBittorrentSettings settings, CancellationToken cancellationToken = default)
    {
        return await ExecuteRequestAsync<Dictionary<string, QBittorrentLabel>>(settings, "/api/v2/torrents/categories", cancellationToken);
    }

    public async Task RemoveTorrentAsync(string hash, bool deleteData, QBittorrentSettings settings, CancellationToken cancellationToken = default)
    {
        var content = new FormUrlEncodedContent(new Dictionary<string, string>
        {
            ["hashes"] = hash.ToLowerInvariant(),
            ["deleteFiles"] = deleteData.ToString().ToLowerInvariant()
        });

        await ExecuteRequestAsync(settings, "/api/v2/torrents/delete", HttpMethod.Post, content, cancellationToken);
    }

    public async Task<bool> TestConnectionAsync(QBittorrentSettings settings, CancellationToken cancellationToken = default)
    {
        try
        {
            await GetApiVersionAsync(settings, cancellationToken);
            return true;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "QBittorrent connection test failed");
            return false;
        }
    }
}
