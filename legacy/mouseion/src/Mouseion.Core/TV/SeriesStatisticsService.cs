// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.TV;

public class SeriesStatistics
{
    public int TotalSeries { get; set; }
    public int MonitoredSeries { get; set; }
    public int TotalEpisodes { get; set; }
    public int EpisodesWithFiles { get; set; }
    public int TotalFiles { get; set; }
    public long TotalSize { get; set; }
}

public interface ISeriesStatisticsService
{
    Task<SeriesStatistics> GetStatisticsAsync(CancellationToken ct = default);
    SeriesStatistics GetStatistics();
}

public class SeriesStatisticsService : ISeriesStatisticsService
{
    private readonly ISeriesRepository _seriesRepository;
    private readonly IEpisodeRepository _episodeRepository;
    private readonly IEpisodeFileRepository _episodeFileRepository;

    public SeriesStatisticsService(
        ISeriesRepository seriesRepository,
        IEpisodeRepository episodeRepository,
        IEpisodeFileRepository episodeFileRepository)
    {
        _seriesRepository = seriesRepository;
        _episodeRepository = episodeRepository;
        _episodeFileRepository = episodeFileRepository;
    }

    public async Task<SeriesStatistics> GetStatisticsAsync(CancellationToken ct = default)
    {
        var allSeries = await _seriesRepository.AllAsync(ct);
        var allSeriesList = allSeries.ToList();
        var monitoredSeries = allSeriesList.Where(s => s.Monitored).ToList();

        var stats = new SeriesStatistics
        {
            TotalSeries = allSeriesList.Count,
            MonitoredSeries = monitoredSeries.Count
        };

        var allEpisodes = await _episodeRepository.AllAsync(ct);
        var allEpisodesList = allEpisodes.ToList();
        stats.TotalEpisodes = allEpisodesList.Count;
        stats.EpisodesWithFiles = allEpisodesList.Count(e => e.EpisodeFileId.HasValue);

        var allFiles = await _episodeFileRepository.AllAsync(ct);
        var allFilesList = allFiles.ToList();

        stats.TotalFiles = allFilesList.Count;
        stats.TotalSize = allFilesList.Sum(f => f.Size);

        return stats;
    }

    public SeriesStatistics GetStatistics()
    {
        var allSeries = _seriesRepository.All().ToList();
        var monitoredSeries = allSeries.Where(s => s.Monitored).ToList();

        var stats = new SeriesStatistics
        {
            TotalSeries = allSeries.Count,
            MonitoredSeries = monitoredSeries.Count
        };

        var allEpisodes = _episodeRepository.All().ToList();
        stats.TotalEpisodes = allEpisodes.Count;
        stats.EpisodesWithFiles = allEpisodes.Count(e => e.EpisodeFileId.HasValue);

        var allFiles = _episodeFileRepository.All().ToList();

        stats.TotalFiles = allFiles.Count;
        stats.TotalSize = allFiles.Sum(f => f.Size);

        return stats;
    }
}
