// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.TV;

namespace Mouseion.Core.MetadataSource.TVDB;

public interface ITVDBProxy
{
    Task<Series?> GetSeriesByTvdbIdAsync(int tvdbId, CancellationToken ct = default);
    Task<List<Episode>> GetEpisodesBySeriesIdAsync(int tvdbId, CancellationToken ct = default);
    Task<List<Series>> SearchSeriesAsync(string query, CancellationToken ct = default);

    Series? GetSeriesByTvdbId(int tvdbId);
    List<Episode> GetEpisodesBySeriesId(int tvdbId);
    List<Series> SearchSeries(string query);
}

public class TVDBProxy : ITVDBProxy
{
    private readonly TVDBSettings _settings;
    private readonly ILogger<TVDBProxy> _logger;
    private readonly HttpClient _httpClient;

    public TVDBProxy(
        TVDBSettings settings,
        ILogger<TVDBProxy> logger,
        HttpClient httpClient)
    {
        _settings = settings;
        _logger = logger;
        _httpClient = httpClient;
        _httpClient.Timeout = TimeSpan.FromSeconds(_settings.TimeoutSeconds);
    }

    public async Task<Series?> GetSeriesByTvdbIdAsync(int tvdbId, CancellationToken ct = default)
    {
        _logger.LogInformation("Fetching series from TVDB: {TvdbId}", tvdbId);

        if (string.IsNullOrWhiteSpace(_settings.ApiKey))
        {
            _logger.LogWarning("TVDB API key not configured");
            return null;
        }

        try
        {
            // TODO: Implement actual TVDB API calls
            _logger.LogWarning("TVDB API integration not yet implemented");
            return null;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error fetching series from TVDB: {TvdbId}", tvdbId);
            return null;
        }
    }

    public Series? GetSeriesByTvdbId(int tvdbId)
    {
        return GetSeriesByTvdbIdAsync(tvdbId).GetAwaiter().GetResult();
    }

    public async Task<List<Episode>> GetEpisodesBySeriesIdAsync(int tvdbId, CancellationToken ct = default)
    {
        _logger.LogInformation("Fetching episodes from TVDB for series: {TvdbId}", tvdbId);

        if (string.IsNullOrWhiteSpace(_settings.ApiKey))
        {
            _logger.LogWarning("TVDB API key not configured");
            return new List<Episode>();
        }

        try
        {
            // TODO: Implement actual TVDB API calls
            _logger.LogWarning("TVDB API integration not yet implemented");
            return new List<Episode>();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error fetching episodes from TVDB for series: {TvdbId}", tvdbId);
            return new List<Episode>();
        }
    }

    public List<Episode> GetEpisodesBySeriesId(int tvdbId)
    {
        return GetEpisodesBySeriesIdAsync(tvdbId).GetAwaiter().GetResult();
    }

    public async Task<List<Series>> SearchSeriesAsync(string query, CancellationToken ct = default)
    {
        _logger.LogInformation("Searching TVDB for: {Query}", query);

        if (string.IsNullOrWhiteSpace(_settings.ApiKey))
        {
            _logger.LogWarning("TVDB API key not configured");
            return new List<Series>();
        }

        try
        {
            // TODO: Implement actual TVDB API calls
            _logger.LogWarning("TVDB API integration not yet implemented");
            return new List<Series>();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching TVDB for: {Query}", query);
            return new List<Series>();
        }
    }

    public List<Series> SearchSeries(string query)
    {
        return SearchSeriesAsync(query).GetAwaiter().GetResult();
    }
}
