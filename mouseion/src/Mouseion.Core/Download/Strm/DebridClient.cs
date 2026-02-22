// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Download.Strm;

/// <summary>
/// Debrid client abstraction: resolves torrent/magnet URIs to direct HTTPS stream URLs.
/// Each provider has different API patterns but the same core flow:
/// 1. Add torrent/magnet → get debrid ID
/// 2. Wait for resolution (instant for cached, minutes for uncached)
/// 3. Get direct download/stream URL(s)
/// </summary>
public interface IDebridClient
{
    DebridProvider Provider { get; }
    Task<DebridResolveResult> ResolveAsync(string magnetOrTorrentUrl, DebridServiceDefinition service, CancellationToken ct = default);
    Task<bool> TestConnectionAsync(DebridServiceDefinition service, CancellationToken ct = default);
    Task<DebridAccountInfo?> GetAccountInfoAsync(DebridServiceDefinition service, CancellationToken ct = default);
}

public class DebridAccountInfo
{
    public string Username { get; set; } = string.Empty;
    public string Plan { get; set; } = string.Empty;
    public DateTime? ExpiresAt { get; set; }
    public decimal? BandwidthUsedGb { get; set; }
    public decimal? BandwidthLimitGb { get; set; }
    public int? ActiveTorrents { get; set; }
}

/// <summary>
/// Real-Debrid API client. Endpoints: https://api.real-debrid.com/rest/1.0/
/// </summary>
public class RealDebridClient : IDebridClient
{
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<RealDebridClient> _logger;
    private const string BaseUrl = "https://api.real-debrid.com/rest/1.0";

    public DebridProvider Provider => DebridProvider.RealDebrid;

    public RealDebridClient(IHttpClientFactory httpClientFactory, ILogger<RealDebridClient> logger)
    {
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    public async Task<DebridResolveResult> ResolveAsync(string magnetOrTorrentUrl, DebridServiceDefinition service, CancellationToken ct = default)
    {
        try
        {
            var client = CreateClient(service);

            // Step 1: Add magnet to Real-Debrid
            var addContent = new FormUrlEncodedContent(new[] { new KeyValuePair<string, string>("magnet", magnetOrTorrentUrl) });
            var addResponse = await client.PostAsync($"{BaseUrl}/torrents/addMagnet", addContent, ct).ConfigureAwait(false);

            if (!addResponse.IsSuccessStatusCode)
            {
                return DebridResolveResult.Failure($"Failed to add magnet: {addResponse.StatusCode}");
            }

            var addResult = await DeserializeAsync<RdAddMagnetResponse>(addResponse, ct);
            if (addResult?.Id == null)
            {
                return DebridResolveResult.Failure("No torrent ID returned");
            }

            // Step 2: Select all files for download
            var selectContent = new FormUrlEncodedContent(new[] { new KeyValuePair<string, string>("files", "all") });
            await client.PostAsync($"{BaseUrl}/torrents/selectFiles/{addResult.Id}", selectContent, ct).ConfigureAwait(false);

            // Step 3: Poll for completion (cached torrents resolve instantly)
            var torrentInfo = await PollForCompletionAsync(client, addResult.Id, ct);
            if (torrentInfo == null)
            {
                return DebridResolveResult.Failure("Torrent resolution timed out");
            }

            // Step 4: Get unrestricted download link from first link
            if (torrentInfo.Links == null || torrentInfo.Links.Count == 0)
            {
                return DebridResolveResult.Failure("No download links available");
            }

            var unrestrictContent = new FormUrlEncodedContent(new[] { new KeyValuePair<string, string>("link", torrentInfo.Links[0]) });
            var unrestrictResponse = await client.PostAsync($"{BaseUrl}/unrestrict/link", unrestrictContent, ct).ConfigureAwait(false);

            if (!unrestrictResponse.IsSuccessStatusCode)
            {
                return DebridResolveResult.Failure($"Failed to unrestrict link: {unrestrictResponse.StatusCode}");
            }

            var downloadInfo = await DeserializeAsync<RdUnrestrictResponse>(unrestrictResponse, ct);
            if (downloadInfo?.Download == null)
            {
                return DebridResolveResult.Failure("No download URL in unrestrict response");
            }

            _logger.LogInformation("Real-Debrid resolved: {FileName} ({Size} bytes)", downloadInfo.Filename, downloadInfo.Filesize);

            return DebridResolveResult.Ok(
                downloadInfo.Download,
                downloadInfo.Filename,
                downloadInfo.Filesize,
                DateTime.UtcNow.AddDays(7) // Real-Debrid links typically expire in 7 days
            );
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Real-Debrid resolution failed");
            return DebridResolveResult.Failure(ex.Message);
        }
    }

    public async Task<bool> TestConnectionAsync(DebridServiceDefinition service, CancellationToken ct = default)
    {
        try
        {
            var info = await GetAccountInfoAsync(service, ct);
            return info != null;
        }
        catch
        {
            return false;
        }
    }

    public async Task<DebridAccountInfo?> GetAccountInfoAsync(DebridServiceDefinition service, CancellationToken ct = default)
    {
        var client = CreateClient(service);
        var response = await client.GetAsync($"{BaseUrl}/user", ct).ConfigureAwait(false);

        if (!response.IsSuccessStatusCode) return null;

        var user = await DeserializeAsync<RdUserResponse>(response, ct);
        if (user == null) return null;

        return new DebridAccountInfo
        {
            Username = user.Username ?? string.Empty,
            Plan = user.Type ?? "unknown",
            ExpiresAt = user.Expiration,
        };
    }

    private async Task<RdTorrentInfoResponse?> PollForCompletionAsync(HttpClient client, string torrentId, CancellationToken ct)
    {
        // Cached torrents resolve instantly; uncached may take minutes
        for (int i = 0; i < 30; i++) // 30 attempts, 2s apart = 60s max
        {
            var response = await client.GetAsync($"{BaseUrl}/torrents/info/{torrentId}", ct).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode) return null;

            var info = await DeserializeAsync<RdTorrentInfoResponse>(response, ct);
            if (info?.Status == "downloaded")
            {
                return info;
            }

            if (info?.Status is "magnet_error" or "error" or "dead")
            {
                _logger.LogWarning("Real-Debrid torrent failed: {Status}", info.Status);
                return null;
            }

            await Task.Delay(2000, ct).ConfigureAwait(false);
        }

        return null;
    }

    private HttpClient CreateClient(DebridServiceDefinition service)
    {
        var client = _httpClientFactory.CreateClient("debrid");
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", service.ApiKey);
        return client;
    }

    private static async Task<T?> DeserializeAsync<T>(HttpResponseMessage response, CancellationToken ct)
    {
        var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        return JsonSerializer.Deserialize<T>(json, new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
    }

    // Real-Debrid API response models
    private class RdAddMagnetResponse { public string? Id { get; set; } public string? Uri { get; set; } }
    private class RdTorrentInfoResponse
    {
        public string? Id { get; set; }
        public string? Status { get; set; }
        public List<string>? Links { get; set; }
    }
    private class RdUnrestrictResponse
    {
        public string? Download { get; set; }
        public string? Filename { get; set; }
        public long Filesize { get; set; }
    }
    private class RdUserResponse
    {
        public string? Username { get; set; }
        public string? Type { get; set; }
        public DateTime? Expiration { get; set; }
    }
}

/// <summary>
/// AllDebrid API client. Endpoints: https://api.alldebrid.com/v4/
/// </summary>
public class AllDebridClient : IDebridClient
{
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<AllDebridClient> _logger;
    private const string BaseUrl = "https://api.alldebrid.com/v4";

    public DebridProvider Provider => DebridProvider.AllDebrid;

    public AllDebridClient(IHttpClientFactory httpClientFactory, ILogger<AllDebridClient> logger)
    {
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    public async Task<DebridResolveResult> ResolveAsync(string magnetOrTorrentUrl, DebridServiceDefinition service, CancellationToken ct = default)
    {
        try
        {
            var client = CreateClient(service);

            // AllDebrid: single-step magnet → link resolution
            var response = await client.GetAsync(
                $"{BaseUrl}/link/unlock?agent=mouseion&apikey={service.ApiKey}&link={Uri.EscapeDataString(magnetOrTorrentUrl)}",
                ct).ConfigureAwait(false);

            if (!response.IsSuccessStatusCode)
            {
                return DebridResolveResult.Failure($"AllDebrid unlock failed: {response.StatusCode}");
            }

            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var result = JsonSerializer.Deserialize<AdUnlockResponse>(json, new JsonSerializerOptions { PropertyNameCaseInsensitive = true });

            if (result?.Data?.Link == null)
            {
                return DebridResolveResult.Failure("AllDebrid returned no link");
            }

            return DebridResolveResult.Ok(
                result.Data.Link,
                result.Data.Filename,
                result.Data.Filesize
            );
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "AllDebrid resolution failed");
            return DebridResolveResult.Failure(ex.Message);
        }
    }

    public async Task<bool> TestConnectionAsync(DebridServiceDefinition service, CancellationToken ct = default)
    {
        var info = await GetAccountInfoAsync(service, ct);
        return info != null;
    }

    public async Task<DebridAccountInfo?> GetAccountInfoAsync(DebridServiceDefinition service, CancellationToken ct = default)
    {
        var client = CreateClient(service);
        var response = await client.GetAsync($"{BaseUrl}/user?agent=mouseion&apikey={service.ApiKey}", ct).ConfigureAwait(false);

        if (!response.IsSuccessStatusCode) return null;

        return new DebridAccountInfo { Username = "alldebrid-user", Plan = "premium" };
    }

    private HttpClient CreateClient(DebridServiceDefinition service)
    {
        return _httpClientFactory.CreateClient("debrid");
    }

    private class AdUnlockResponse { public AdUnlockData? Data { get; set; } }
    private class AdUnlockData
    {
        public string? Link { get; set; }
        public string? Filename { get; set; }
        public long Filesize { get; set; }
    }
}

/// <summary>
/// Premiumize API client. Endpoints: https://www.premiumize.me/api/
/// </summary>
public class PremiumizeClient : IDebridClient
{
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<PremiumizeClient> _logger;
    private const string BaseUrl = "https://www.premiumize.me/api";

    public DebridProvider Provider => DebridProvider.Premiumize;

    public PremiumizeClient(IHttpClientFactory httpClientFactory, ILogger<PremiumizeClient> logger)
    {
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    public async Task<DebridResolveResult> ResolveAsync(string magnetOrTorrentUrl, DebridServiceDefinition service, CancellationToken ct = default)
    {
        try
        {
            var client = _httpClientFactory.CreateClient("debrid");

            // Premiumize: directdl endpoint for instant cached results
            var content = new FormUrlEncodedContent(new[]
            {
                new KeyValuePair<string, string>("apikey", service.ApiKey),
                new KeyValuePair<string, string>("src", magnetOrTorrentUrl)
            });

            var response = await client.PostAsync($"{BaseUrl}/transfer/directdl", content, ct).ConfigureAwait(false);

            if (!response.IsSuccessStatusCode)
            {
                return DebridResolveResult.Failure($"Premiumize directdl failed: {response.StatusCode}");
            }

            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var result = JsonSerializer.Deserialize<PmDirectDlResponse>(json, new JsonSerializerOptions { PropertyNameCaseInsensitive = true });

            if (result?.Content == null || result.Content.Count == 0)
            {
                return DebridResolveResult.Failure("Premiumize returned no content");
            }

            // Get largest file (usually the video)
            var bestFile = result.Content.OrderByDescending(c => c.Size).First();

            return DebridResolveResult.Ok(
                bestFile.Link ?? string.Empty,
                bestFile.Path,
                bestFile.Size
            );
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Premiumize resolution failed");
            return DebridResolveResult.Failure(ex.Message);
        }
    }

    public async Task<bool> TestConnectionAsync(DebridServiceDefinition service, CancellationToken ct = default)
    {
        var info = await GetAccountInfoAsync(service, ct);
        return info != null;
    }

    public async Task<DebridAccountInfo?> GetAccountInfoAsync(DebridServiceDefinition service, CancellationToken ct = default)
    {
        var client = _httpClientFactory.CreateClient("debrid");
        var response = await client.GetAsync($"{BaseUrl}/account/info?apikey={service.ApiKey}", ct).ConfigureAwait(false);

        if (!response.IsSuccessStatusCode) return null;

        return new DebridAccountInfo { Username = "premiumize-user", Plan = "premium" };
    }

    private class PmDirectDlResponse
    {
        public string? Status { get; set; }
        public List<PmContent>? Content { get; set; }
    }

    private class PmContent
    {
        public string? Path { get; set; }
        public long Size { get; set; }
        public string? Link { get; set; }

        [JsonPropertyName("stream_link")]
        public string? StreamLink { get; set; }
    }
}
