// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Movies.Calendar;

public class MovieCalendarService : IMovieCalendarService
{
    private readonly IMovieRepository _movieRepository;
    private readonly ILogger<MovieCalendarService> _logger;

    public MovieCalendarService(IMovieRepository movieRepository, ILogger<MovieCalendarService> logger)
    {
        _movieRepository = movieRepository;
        _logger = logger;
    }

    public async Task<List<MovieCalendarEntry>> GetCalendarEntriesAsync(DateTime start, DateTime end, bool includeUnmonitored = false, CancellationToken ct = default)
    {
        _logger.LogDebug("Getting calendar entries from {Start} to {End}, includeUnmonitored: {IncludeUnmonitored}", start, end, includeUnmonitored);

        var movies = await _movieRepository.GetMoviesBetweenDatesAsync(start, end, includeUnmonitored, ct).ConfigureAwait(false);
        var entries = new List<MovieCalendarEntry>();

        foreach (var movie in movies)
        {
            if (movie.InCinemas.HasValue && movie.InCinemas.Value >= start && movie.InCinemas.Value <= end)
            {
                entries.Add(new MovieCalendarEntry
                {
                    MovieId = movie.Id,
                    Title = movie.Title,
                    Year = movie.Year,
                    ReleaseDate = movie.InCinemas.Value,
                    ReleaseType = "Cinema",
                    Monitored = movie.Monitored,
                    HasFile = false // TODO: Check if movie has file
                });
            }

            if (movie.DigitalRelease.HasValue && movie.DigitalRelease.Value >= start && movie.DigitalRelease.Value <= end)
            {
                entries.Add(new MovieCalendarEntry
                {
                    MovieId = movie.Id,
                    Title = movie.Title,
                    Year = movie.Year,
                    ReleaseDate = movie.DigitalRelease.Value,
                    ReleaseType = "Digital",
                    Monitored = movie.Monitored,
                    HasFile = false // TODO: Check if movie has file
                });
            }

            if (movie.PhysicalRelease.HasValue && movie.PhysicalRelease.Value >= start && movie.PhysicalRelease.Value <= end)
            {
                entries.Add(new MovieCalendarEntry
                {
                    MovieId = movie.Id,
                    Title = movie.Title,
                    Year = movie.Year,
                    ReleaseDate = movie.PhysicalRelease.Value,
                    ReleaseType = "Physical",
                    Monitored = movie.Monitored,
                    HasFile = false // TODO: Check if movie has file
                });
            }
        }

        return entries.OrderBy(e => e.ReleaseDate).ToList();
    }

    public List<MovieCalendarEntry> GetCalendarEntries(DateTime start, DateTime end, bool includeUnmonitored = false)
    {
        _logger.LogDebug("Getting calendar entries from {Start} to {End}, includeUnmonitored: {IncludeUnmonitored}", start, end, includeUnmonitored);

        var movies = _movieRepository.GetMoviesBetweenDates(start, end, includeUnmonitored);
        var entries = new List<MovieCalendarEntry>();

        foreach (var movie in movies)
        {
            if (movie.InCinemas.HasValue && movie.InCinemas.Value >= start && movie.InCinemas.Value <= end)
            {
                entries.Add(new MovieCalendarEntry
                {
                    MovieId = movie.Id,
                    Title = movie.Title,
                    Year = movie.Year,
                    ReleaseDate = movie.InCinemas.Value,
                    ReleaseType = "Cinema",
                    Monitored = movie.Monitored,
                    HasFile = false // TODO: Check if movie has file
                });
            }

            if (movie.DigitalRelease.HasValue && movie.DigitalRelease.Value >= start && movie.DigitalRelease.Value <= end)
            {
                entries.Add(new MovieCalendarEntry
                {
                    MovieId = movie.Id,
                    Title = movie.Title,
                    Year = movie.Year,
                    ReleaseDate = movie.DigitalRelease.Value,
                    ReleaseType = "Digital",
                    Monitored = movie.Monitored,
                    HasFile = false // TODO: Check if movie has file
                });
            }

            if (movie.PhysicalRelease.HasValue && movie.PhysicalRelease.Value >= start && movie.PhysicalRelease.Value <= end)
            {
                entries.Add(new MovieCalendarEntry
                {
                    MovieId = movie.Id,
                    Title = movie.Title,
                    Year = movie.Year,
                    ReleaseDate = movie.PhysicalRelease.Value,
                    ReleaseType = "Physical",
                    Monitored = movie.Monitored,
                    HasFile = false // TODO: Check if movie has file
                });
            }
        }

        return entries.OrderBy(e => e.ReleaseDate).ToList();
    }
}
