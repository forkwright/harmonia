// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Manga;

namespace Mouseion.Core.HealthCheck.Checks;

public class MangaLibraryHealthCheck : IProvideHealthCheck
{
    private readonly IMangaSeriesRepository _seriesRepository;
    private readonly IMangaChapterRepository _chapterRepository;

    public MangaLibraryHealthCheck(
        IMangaSeriesRepository seriesRepository,
        IMangaChapterRepository chapterRepository)
    {
        _seriesRepository = seriesRepository;
        _chapterRepository = chapterRepository;
    }

    public HealthCheck Check()
    {
        var monitoredSeries = _seriesRepository.GetMonitored();

        if (monitoredSeries.Count == 0)
        {
            return new HealthCheck(
                HealthCheckResult.Ok,
                "No manga series configured"
            );
        }

        var seriesWithoutChapters = new List<string>();

        foreach (var series in monitoredSeries)
        {
            var chapters = _chapterRepository.GetBySeriesId(series.Id);
            if (chapters.Count == 0)
            {
                seriesWithoutChapters.Add(series.Title);
            }
        }

        if (seriesWithoutChapters.Count > 0)
        {
            var seriesList = seriesWithoutChapters.Count <= 3
                ? string.Join(", ", seriesWithoutChapters)
                : $"{seriesWithoutChapters[0]}, {seriesWithoutChapters[1]} and {seriesWithoutChapters.Count - 2} more";

            return new HealthCheck(
                HealthCheckResult.Warning,
                $"Manga series without chapters: {seriesList}",
                "manga-series-missing-chapters"
            );
        }

        return new HealthCheck(
            HealthCheckResult.Ok,
            $"All {monitoredSeries.Count} manga series have chapters"
        );
    }
}
