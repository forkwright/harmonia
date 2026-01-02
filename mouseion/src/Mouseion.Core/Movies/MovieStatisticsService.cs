// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies;

public class MovieStatistics
{
    public int TotalMovies { get; set; }
    public int MonitoredMovies { get; set; }
    public int MoviesWithFiles { get; set; }
    public int TotalFiles { get; set; }
    public long TotalSize { get; set; }
}

public interface IMovieStatisticsService
{
    Task<MovieStatistics> GetStatisticsAsync(CancellationToken ct = default);
    MovieStatistics GetStatistics();
}

public class MovieStatisticsService : IMovieStatisticsService
{
    private readonly IMovieRepository _movieRepository;
    private readonly IMovieFileRepository _movieFileRepository;

    public MovieStatisticsService(
        IMovieRepository movieRepository,
        IMovieFileRepository movieFileRepository)
    {
        _movieRepository = movieRepository;
        _movieFileRepository = movieFileRepository;
    }

    public async Task<MovieStatistics> GetStatisticsAsync(CancellationToken ct = default)
    {
        var allMovies = await _movieRepository.AllAsync(ct);
        var allMoviesList = allMovies.ToList();
        var monitoredMovies = allMoviesList.Where(m => m.Monitored).ToList();

        var stats = new MovieStatistics
        {
            TotalMovies = allMoviesList.Count,
            MonitoredMovies = monitoredMovies.Count
        };

        var allFiles = await _movieFileRepository.AllAsync(ct);
        var allFilesList = allFiles.ToList();

        stats.TotalFiles = allFilesList.Count;
        stats.TotalSize = allFilesList.Sum(f => f.Size);
        stats.MoviesWithFiles = allFilesList.Where(f => f.MovieId.HasValue).Select(f => f.MovieId).Distinct().Count();

        return stats;
    }

    public MovieStatistics GetStatistics()
    {
        var allMovies = _movieRepository.All().ToList();
        var monitoredMovies = allMovies.Where(m => m.Monitored).ToList();

        var stats = new MovieStatistics
        {
            TotalMovies = allMovies.Count,
            MonitoredMovies = monitoredMovies.Count
        };

        var allFiles = _movieFileRepository.All().ToList();

        stats.TotalFiles = allFiles.Count;
        stats.TotalSize = allFiles.Sum(f => f.Size);
        stats.MoviesWithFiles = allFiles.Where(f => f.MovieId.HasValue).Select(f => f.MovieId).Distinct().Count();

        return stats;
    }
}
