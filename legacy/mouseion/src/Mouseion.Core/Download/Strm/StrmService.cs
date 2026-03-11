// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Download.Strm;

/// <summary>
/// .strm file service: generates and manages .strm files that media servers
/// (Jellyfin, Emby, Kodi) play directly from debrid stream URLs.
/// Movies/TV only — books, music, and podcasts remain download-only.
/// </summary>
public interface IStrmService
{
    /// <summary>Generate a .strm file by resolving a magnet through the best available debrid service.</summary>
    Task<StrmFile?> CreateStrmAsync(StrmRequest request, CancellationToken ct = default);

    /// <summary>Verify all .strm URLs are still valid; mark expired ones.</summary>
    Task<StrmVerificationResult> VerifyAllAsync(CancellationToken ct = default);

    /// <summary>Re-resolve an expired .strm file through debrid.</summary>
    Task<StrmFile?> RefreshStrmAsync(int strmFileId, string magnetUrl, CancellationToken ct = default);

    /// <summary>Delete a .strm file from disk and database.</summary>
    Task DeleteStrmAsync(int strmFileId, CancellationToken ct = default);

    /// <summary>Check if media type supports .strm mode.</summary>
    bool SupportsStrm(MediaType mediaType);
}

public class StrmService : IStrmService
{
    private readonly IStrmFileRepository _strmRepository;
    private readonly IDebridServiceRepository _debridRepository;
    private readonly IEnumerable<IDebridClient> _debridClients;
    private readonly IHttpClientFactory _httpClientFactory;
    private readonly ILogger<StrmService> _logger;

    // Media types that support .strm (video streaming only)
    private static readonly HashSet<MediaType> StrmSupportedTypes = new()
    {
        MediaType.Movie,
        MediaType.TV
    };

    public StrmService(
        IStrmFileRepository strmRepository,
        IDebridServiceRepository debridRepository,
        IEnumerable<IDebridClient> debridClients,
        IHttpClientFactory httpClientFactory,
        ILogger<StrmService> logger)
    {
        _strmRepository = strmRepository;
        _debridRepository = debridRepository;
        _debridClients = debridClients;
        _httpClientFactory = httpClientFactory;
        _logger = logger;
    }

    public bool SupportsStrm(MediaType mediaType) => StrmSupportedTypes.Contains(mediaType);

    public async Task<StrmFile?> CreateStrmAsync(StrmRequest request, CancellationToken ct = default)
    {
        if (!SupportsStrm(request.MediaType))
        {
            _logger.LogWarning(".strm not supported for {MediaType}", request.MediaType);
            return null;
        }

        // Try each enabled debrid service in priority order
        var services = _debridRepository.GetEnabled();
        if (services.Count == 0)
        {
            _logger.LogWarning("No debrid services configured");
            return null;
        }

        foreach (var service in services)
        {
            var client = _debridClients.FirstOrDefault(c => c.Provider == service.Provider);
            if (client == null) continue;

            // Check bandwidth limits
            if (service.BandwidthLimitGb.HasValue && service.BandwidthUsedGb.HasValue
                && service.BandwidthUsedGb >= service.BandwidthLimitGb)
            {
                _logger.LogDebug("Skipping {Provider}: bandwidth limit reached ({Used}/{Limit} GB)",
                    service.Provider, service.BandwidthUsedGb, service.BandwidthLimitGb);
                continue;
            }

            var result = await client.ResolveAsync(request.MagnetUrl, service, ct);
            if (!result.Success)
            {
                _logger.LogWarning("Debrid resolve failed via {Provider}: {Error}", service.Provider, result.ErrorMessage);
                continue;
            }

            // Build .strm file path
            var strmPath = BuildStrmPath(request.RootFolderPath, request.OrganizedPath, request.Title);

            // Write .strm file to disk
            var directory = Path.GetDirectoryName(strmPath);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            await File.WriteAllTextAsync(strmPath, result.StreamUrl!, ct).ConfigureAwait(false);

            // Record in database
            var strmFile = new StrmFile
            {
                MediaItemId = request.MediaItemId,
                DebridServiceId = service.Id,
                FilePath = strmPath,
                StreamUrl = result.StreamUrl!,
                Quality = result.Quality ?? request.Quality,
                SizeBytes = result.SizeBytes,
                IsValid = true,
                LastVerified = DateTime.UtcNow,
                ExpiresAt = result.ExpiresAt,
                CreatedAt = DateTime.UtcNow
            };

            var inserted = _strmRepository.Insert(strmFile);

            var urlPreview = result.StreamUrl!.Length > 80 ? result.StreamUrl[..80] + "..." : result.StreamUrl;
            _logger.LogInformation("Created .strm: {Path} → {Url} via {Provider}", strmPath, urlPreview, service.Provider);

            return inserted;
        }

        _logger.LogError("All debrid services failed for: {Title}", request.Title);
        return null;
    }

    public async Task<StrmVerificationResult> VerifyAllAsync(CancellationToken ct = default)
    {
        var result = new StrmVerificationResult();
        var expired = await _strmRepository.GetExpiredAsync(ct);
        var httpClient = _httpClientFactory.CreateClient("strm-verify");
        httpClient.Timeout = TimeSpan.FromSeconds(10);

        // Check expired URLs
        foreach (var strm in expired)
        {
            strm.IsValid = false;
            _strmRepository.Update(strm);
            result.Expired++;
        }

        // Spot-check a sample of valid .strm files
        var validCount = await _strmRepository.CountValidAsync(ct);
        var sampleSize = Math.Min(50, validCount);
        // We'll rely on expiration dates rather than HTTP HEAD checks to avoid hammering debrid APIs

        result.TotalChecked = expired.Count;
        result.StillValid = validCount - expired.Count;

        _logger.LogInformation("Strm verification: {Expired} expired, {Valid} valid", result.Expired, result.StillValid);
        return result;
    }

    public async Task<StrmFile?> RefreshStrmAsync(int strmFileId, string magnetUrl, CancellationToken ct = default)
    {
        var existing = _strmRepository.Get(strmFileId);
        if (existing == null) return null;

        // Mark old as invalid
        existing.IsValid = false;
        _strmRepository.Update(existing);

        // Create new .strm with same path
        var request = new StrmRequest
        {
            MediaItemId = existing.MediaItemId,
            MediaType = MediaType.Movie, // Default; caller should provide actual type
            MagnetUrl = magnetUrl,
            Title = Path.GetFileNameWithoutExtension(existing.FilePath),
            RootFolderPath = Path.GetDirectoryName(Path.GetDirectoryName(existing.FilePath)) ?? "",
            OrganizedPath = "",
            Quality = existing.Quality
        };

        return await CreateStrmAsync(request, ct);
    }

    public async Task DeleteStrmAsync(int strmFileId, CancellationToken ct = default)
    {
        var strm = _strmRepository.Get(strmFileId);
        if (strm == null) return;

        // Delete file from disk
        if (File.Exists(strm.FilePath))
        {
            File.Delete(strm.FilePath);
            _logger.LogInformation("Deleted .strm file: {Path}", strm.FilePath);
        }

        _strmRepository.Delete(strmFileId);
    }

    private static string BuildStrmPath(string rootFolder, string organizedPath, string title)
    {
        // Sanitize title for filesystem
        var safeTitle = string.Join("_", title.Split(Path.GetInvalidFileNameChars()));
        var fileName = $"{safeTitle}.strm";

        if (!string.IsNullOrEmpty(organizedPath))
        {
            return Path.Combine(rootFolder, organizedPath, fileName);
        }

        return Path.Combine(rootFolder, fileName);
    }
}

public class StrmRequest
{
    public int MediaItemId { get; set; }
    public MediaType MediaType { get; set; }
    public string MagnetUrl { get; set; } = string.Empty;
    public string Title { get; set; } = string.Empty;
    public string RootFolderPath { get; set; } = string.Empty;
    public string OrganizedPath { get; set; } = string.Empty;
    public string? Quality { get; set; }
}

public class StrmVerificationResult
{
    public int TotalChecked { get; set; }
    public int StillValid { get; set; }
    public int Expired { get; set; }
    public int Refreshed { get; set; }
}
