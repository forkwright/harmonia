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
    private readonly IMovieFileRepository _movieFileRepository;
    private readonly ILogger<MovieCalendarService> _logger;

    public MovieCalendarService(
        IMovieRepository movieRepository,
        IMovieFileRepository movieFileRepository,
        ILogger<MovieCalendarService> logger)
    {
        _movieRepository = movieRepository;
        _movieFileRepository = movieFileRepository;
        _logger = logger;
    }

    public async Task<List<MovieCalendarEntry>> GetCalendarEntriesAsync(DateTime start, DateTime end, bool includeUnmonitored = false, CancellationToken ct = default)
    {
        _logger.LogDebug("Getting calendar entries from {Start} to {End}, includeUnmonitored: {IncludeUnmonitored}", start, end, includeUnmonitored);

        var movies = await _movieRepository.GetMoviesBetweenDatesAsync(start, end, includeUnmonitored, ct).ConfigureAwait(false);

        // Bulk check file existence to avoid N+1 queries
        var movieIds = movies.Select(m => m.Id).Distinct().ToList();
        var moviesWithFiles = new HashSet<int>();

        foreach (var movieId in movieIds)
        {
            var file = await _movieFileRepository.FindByMovieIdAsync(movieId, ct).ConfigureAwait(false);
            if (file != null)
            {
                moviesWithFiles.Add(movieId);
            }
        }

        return BuildCalendarEntries(movies, start, end, moviesWithFiles);
    }

    public List<MovieCalendarEntry> GetCalendarEntries(DateTime start, DateTime end, bool includeUnmonitored = false)
    {
        _logger.LogDebug("Getting calendar entries from {Start} to {End}, includeUnmonitored: {IncludeUnmonitored}", start, end, includeUnmonitored);

        var movies = _movieRepository.GetMoviesBetweenDates(start, end, includeUnmonitored);

        // Bulk check file existence to avoid N+1 queries
        var movieIds = movies.Select(m => m.Id).Distinct().ToList();
        var moviesWithFiles = new HashSet<int>();

        foreach (var movieId in movieIds)
        {
            var file = _movieFileRepository.FindByMovieId(movieId);
            if (file != null)
            {
                moviesWithFiles.Add(movieId);
            }
        }

        return BuildCalendarEntries(movies, start, end, moviesWithFiles);
    }

    private List<MovieCalendarEntry> BuildCalendarEntries(
        IEnumerable<Movie> movies,
        DateTime start,
        DateTime end,
        HashSet<int> moviesWithFiles)
    {
        var entries = new List<MovieCalendarEntry>();

        foreach (var movie in movies)
        {
            var hasFile = moviesWithFiles.Contains(movie.Id);

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
                    HasFile = hasFile
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
                    HasFile = hasFile
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
                    HasFile = hasFile
                });
            }
        }

        return entries.OrderBy(e => e.ReleaseDate).ToList();
    }
}
