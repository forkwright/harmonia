// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.TV;

public interface IAddSeriesService
{
    Task<Series> AddSeriesAsync(Series series, CancellationToken ct = default);
    Task<List<Series>> AddSeriesListAsync(List<Series> seriesList, CancellationToken ct = default);

    Series AddSeries(Series series);
    List<Series> AddSeriesList(List<Series> seriesList);
}

public partial class AddSeriesService : IAddSeriesService
{
    private readonly ISeriesRepository _seriesRepository;
    private readonly ILogger<AddSeriesService> _logger;

    public AddSeriesService(
        ISeriesRepository seriesRepository,
        ILogger<AddSeriesService> logger)
    {
        _seriesRepository = seriesRepository;
        _logger = logger;
    }

    public async Task<Series> AddSeriesAsync(Series series, CancellationToken ct = default)
    {
        ValidateSeries(series);

        if (series.TvdbId.HasValue)
        {
            var existing = await _seriesRepository.FindByTvdbIdAsync(series.TvdbId.Value, ct).ConfigureAwait(false);
            if (existing != null)
            {
                LogSeriesExistsWithTvdbId(series.Title.SanitizeForLog(), series.Year, series.TvdbId);
                return existing;
            }
        }

        if (!string.IsNullOrWhiteSpace(series.CleanTitle))
        {
            var existing = await _seriesRepository.FindByTitleAsync(series.CleanTitle, ct).ConfigureAwait(false);
            if (existing != null)
            {
                LogSeriesExists(series.Title.SanitizeForLog(), series.Year);
                return existing;
            }
        }

        series.Added = DateTime.UtcNow;
        series.Monitored = true;

        var added = await _seriesRepository.InsertAsync(series, ct).ConfigureAwait(false);
        LogSeriesAdded(added.Title.SanitizeForLog(), added.Year, added.TvdbId);

        return added;
    }

    public Series AddSeries(Series series)
    {
        ValidateSeries(series);

        if (series.TvdbId.HasValue)
        {
            var existing = _seriesRepository.FindByTvdbId(series.TvdbId.Value);
            if (existing != null)
            {
                LogSeriesExistsWithTvdbId(series.Title.SanitizeForLog(), series.Year, series.TvdbId);
                return existing;
            }
        }

        if (!string.IsNullOrWhiteSpace(series.CleanTitle))
        {
            var existing = _seriesRepository.FindByTitle(series.CleanTitle);
            if (existing != null)
            {
                LogSeriesExists(series.Title.SanitizeForLog(), series.Year);
                return existing;
            }
        }

        series.Added = DateTime.UtcNow;
        series.Monitored = true;

        var added = _seriesRepository.Insert(series);
        LogSeriesAdded(added.Title.SanitizeForLog(), added.Year, added.TvdbId);

        return added;
    }

    public async Task<List<Series>> AddSeriesListAsync(List<Series> seriesList, CancellationToken ct = default)
    {
        var addedSeries = new List<Series>();

        foreach (var series in seriesList)
        {
            try
            {
                var added = await AddSeriesAsync(series, ct).ConfigureAwait(false);
                addedSeries.Add(added);
            }
            catch (ArgumentException ex)
            {
                LogValidationFailed(ex, series.Title.SanitizeForLog(), series.Year);
            }
            catch (InvalidOperationException ex)
            {
                LogAddingError(ex, series.Title.SanitizeForLog(), series.Year);
            }
        }

        return addedSeries;
    }

    public List<Series> AddSeriesList(List<Series> seriesList)
    {
        var addedSeries = new List<Series>();

        foreach (var series in seriesList)
        {
            try
            {
                var added = AddSeries(series);
                addedSeries.Add(added);
            }
            catch (ArgumentException ex)
            {
                LogValidationFailed(ex, series.Title.SanitizeForLog(), series.Year);
            }
            catch (InvalidOperationException ex)
            {
                LogAddingError(ex, series.Title.SanitizeForLog(), series.Year);
            }
        }

        return addedSeries;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Series already exists: {SeriesTitle} ({Year}) - TVDB ID: {TvdbId}")]
    private partial void LogSeriesExistsWithTvdbId(string seriesTitle, int year, int? tvdbId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Series already exists: {SeriesTitle} ({Year})")]
    private partial void LogSeriesExists(string seriesTitle, int year);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added series: {SeriesTitle} ({Year}) - TVDB ID: {TvdbId}")]
    private partial void LogSeriesAdded(string seriesTitle, int year, int? tvdbId);

    [LoggerMessage(Level = LogLevel.Error, Message = "Validation failed for series: {SeriesTitle} ({Year})")]
    private partial void LogValidationFailed(Exception ex, string seriesTitle, int year);

    [LoggerMessage(Level = LogLevel.Error, Message = "Error adding series: {SeriesTitle} ({Year})")]
    private partial void LogAddingError(Exception ex, string seriesTitle, int year);

    private void ValidateSeries(Series series)
    {
        if (string.IsNullOrWhiteSpace(series.Title))
        {
            throw new ArgumentException("Series title is required", nameof(series));
        }

        if (series.Year <= 0)
        {
            throw new ArgumentException("Series year must be greater than 0", nameof(series));
        }

        if (series.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(series));
        }

        if (string.IsNullOrWhiteSpace(series.Path))
        {
            throw new ArgumentException("Series path is required", nameof(series));
        }
    }
}
