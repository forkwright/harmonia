// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Movies.Monitoring;

public class ReleaseMonitoringService : IReleaseMonitoringService
{
    private readonly IMovieRepository _movieRepository;
    private readonly ILogger<ReleaseMonitoringService> _logger;

    public ReleaseMonitoringService(IMovieRepository movieRepository, ILogger<ReleaseMonitoringService> logger)
    {
        _movieRepository = movieRepository;
        _logger = logger;
    }

    public async Task<List<Movie>> GetUpcomingReleasesAsync(int daysAhead = 30, CancellationToken ct = default)
    {
        var now = DateTime.UtcNow;
        var future = now.AddDays(daysAhead);

        _logger.LogDebug("Getting upcoming releases from {Now} to {Future}", now, future);

        var movies = await _movieRepository.GetMoviesBetweenDatesAsync(now, future, includeUnmonitored: false, ct).ConfigureAwait(false);

        return movies.OrderBy(m => GetEarliestReleaseDate(m)).ToList();
    }

    public async Task<List<Movie>> GetRecentReleasesAsync(int daysBehind = 7, CancellationToken ct = default)
    {
        var now = DateTime.UtcNow;
        var past = now.AddDays(-daysBehind);

        _logger.LogDebug("Getting recent releases from {Past} to {Now}", past, now);

        var movies = await _movieRepository.GetMoviesBetweenDatesAsync(past, now, includeUnmonitored: false, ct).ConfigureAwait(false);

        return movies.OrderByDescending(m => GetEarliestReleaseDate(m)).ToList();
    }

    private static DateTime? GetEarliestReleaseDate(Movie movie)
    {
        var dates = new List<DateTime?> { movie.InCinemas, movie.DigitalRelease, movie.PhysicalRelease };
        return dates.Where(d => d.HasValue).OrderBy(d => d!.Value).FirstOrDefault();
    }
}
