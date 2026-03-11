// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Clients.NZBGet;

/// <summary>
/// Low-level NZBGet JSON-RPC proxy. Auth via HTTP Basic over the JSON-RPC endpoint.
/// </summary>
public class NZBGetProxy
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<NZBGetProxy> _logger;

    public NZBGetProxy(IHttpClientFactory httpClientFactory, ILogger<NZBGetProxy> logger)
    {
        _httpClient = httpClientFactory.CreateClient("NZBGet");
        _logger = logger;
    }

    private string BuildUrl(NZBGetSettings settings)
    {
        var protocol = settings.UseSsl ? "https" : "http";
        var urlBase = string.IsNullOrWhiteSpace(settings.UrlBase)
            ? string.Empty
            : $"/{settings.UrlBase.Trim('/')}";

        return $"{protocol}://{settings.Host}:{settings.Port}{urlBase}/jsonrpc";
    }

    private async Task<T> ExecuteRpcAsync<T>(
        NZBGetSettings settings,
        string method,
        object[]? parameters = null,
        CancellationToken cancellationToken = default)
    {
        var url = BuildUrl(settings);
        var request = new JsonRpcRequest
        {
            Method = method,
            Params = parameters ?? Array.Empty<object>()
        };

        var json = JsonSerializer.Serialize(request);
        using var httpRequest = new HttpRequestMessage(HttpMethod.Post, url);
        httpRequest.Content = new StringContent(json, Encoding.UTF8, "application/json");

        // NZBGet uses HTTP Basic auth
        var credentials = Convert.ToBase64String(
            Encoding.UTF8.GetBytes($"{settings.Username}:{settings.Password}"));
        httpRequest.Headers.Authorization = new AuthenticationHeaderValue("Basic", credentials);

        using var response = await _httpClient.SendAsync(httpRequest, cancellationToken);
        response.EnsureSuccessStatusCode();

        var responseText = await response.Content.ReadAsStringAsync(cancellationToken);
        var rpcResponse = JsonSerializer.Deserialize<JsonRpcResponse<T>>(responseText);

        if (rpcResponse?.Error != null)
        {
            throw new HttpRequestException(
                $"NZBGet RPC error {rpcResponse.Error.Code}: {rpcResponse.Error.Message}");
        }
        return rpcResponse != null ? rpcResponse.Result : throw new InvalidOperationException("NZBGet returned null result");


    }

    public async Task<List<NZBGetQueueItem>> GetQueueAsync(
        NZBGetSettings settings,
        CancellationToken cancellationToken = default)
    {
        return await ExecuteRpcAsync<List<NZBGetQueueItem>>(settings, "listgroups", cancellationToken: cancellationToken);
    }

    public async Task<List<NZBGetHistoryItem>> GetHistoryAsync(
        NZBGetSettings settings,
        CancellationToken cancellationToken = default)
    {
        // hidden=true shows all items including hidden
        return await ExecuteRpcAsync<List<NZBGetHistoryItem>>(settings, "history",
            new object[] { false }, cancellationToken);
    }

    public async Task<NZBGetStatus> GetStatusAsync(
        NZBGetSettings settings,
        CancellationToken cancellationToken = default)
    {
        return await ExecuteRpcAsync<NZBGetStatus>(settings, "status", cancellationToken: cancellationToken);
    }

    public async Task<string> GetVersionAsync(
        NZBGetSettings settings,
        CancellationToken cancellationToken = default)
    {
        return await ExecuteRpcAsync<string>(settings, "version", cancellationToken: cancellationToken);
    }

    public async Task DeleteAsync(
        int nzbId,
        bool deleteData,
        NZBGetSettings settings,
        CancellationToken cancellationToken = default)
    {
        // editqueue: GroupDelete (for active) or HistoryDelete (for history)
        // Try queue first, then history
        try
        {
            await ExecuteRpcAsync<bool>(settings, "editqueue",
                new object[] { "GroupDelete", "", new[] { nzbId } }, cancellationToken);
        }
        catch
        {
            await ExecuteRpcAsync<bool>(settings, "editqueue",
                new object[] { deleteData ? "HistoryFinalDelete" : "HistoryDelete", "", new[] { nzbId } },
                cancellationToken);
        }
    }

    public async Task<bool> TestConnectionAsync(
        NZBGetSettings settings,
        CancellationToken cancellationToken = default)
    {
        try
        {
            var version = await GetVersionAsync(settings, cancellationToken);
            _logger.LogDebug("Connected to NZBGet {Version}", version);
            return true;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error testing NZBGet connection");
            return false;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse NZBGet response during connection test");
            return false;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out testing NZBGet connection");
            return false;
        }
    }
}
