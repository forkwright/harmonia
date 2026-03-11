// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Webcomic;

namespace Mouseion.Core.HealthCheck.Checks;

public class WebcomicLibraryHealthCheck : IProvideHealthCheck
{
    private readonly IWebcomicSeriesRepository _seriesRepository;
    private readonly IWebcomicEpisodeRepository _episodeRepository;

    public WebcomicLibraryHealthCheck(
        IWebcomicSeriesRepository seriesRepository,
        IWebcomicEpisodeRepository episodeRepository)
    {
        _seriesRepository = seriesRepository;
        _episodeRepository = episodeRepository;
    }

    public HealthCheck Check()
    {
        var monitoredSeries = _seriesRepository.GetMonitored();

        if (monitoredSeries.Count == 0)
        {
            return new HealthCheck(
                HealthCheckResult.Ok,
                "No webcomic series configured"
            );
        }

        var seriesWithoutEpisodes = new List<string>();

        foreach (var series in monitoredSeries)
        {
            var episodes = _episodeRepository.GetBySeriesId(series.Id);
            if (episodes.Count == 0)
            {
                seriesWithoutEpisodes.Add(series.Title);
            }
        }

        if (seriesWithoutEpisodes.Count > 0)
        {
            var seriesList = seriesWithoutEpisodes.Count <= 3
                ? string.Join(", ", seriesWithoutEpisodes)
                : $"{seriesWithoutEpisodes[0]}, {seriesWithoutEpisodes[1]} and {seriesWithoutEpisodes.Count - 2} more";

            return new HealthCheck(
                HealthCheckResult.Warning,
                $"Webcomic series without episodes: {seriesList}",
                "webcomic-series-missing-episodes"
            );
        }

        return new HealthCheck(
            HealthCheckResult.Ok,
            $"All {monitoredSeries.Count} webcomic series have episodes"
        );
    }
}
