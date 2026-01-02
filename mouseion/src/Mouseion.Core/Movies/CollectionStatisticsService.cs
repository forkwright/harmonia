// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies;

public class CollectionStatistics
{
    public int CollectionId { get; set; }
    public int MovieCount { get; set; }
    public int MonitoredMovieCount { get; set; }
    public int MovieFileCount { get; set; }
    public long SizeOnDisk { get; set; }
}

public interface ICollectionStatisticsService
{
    Task<CollectionStatistics> GetStatisticsAsync(int collectionId, CancellationToken ct = default);
    CollectionStatistics GetStatistics(int collectionId);
}

public class CollectionStatisticsService : ICollectionStatisticsService
{
    private readonly IMovieRepository _movieRepository;
    private readonly IMovieFileRepository _movieFileRepository;

    public CollectionStatisticsService(
        IMovieRepository movieRepository,
        IMovieFileRepository movieFileRepository)
    {
        _movieRepository = movieRepository;
        _movieFileRepository = movieFileRepository;
    }

    public async Task<CollectionStatistics> GetStatisticsAsync(int collectionId, CancellationToken ct = default)
    {
        var movies = await _movieRepository.GetByCollectionIdAsync(collectionId, ct);

        var stats = new CollectionStatistics
        {
            CollectionId = collectionId,
            MovieCount = movies.Count,
            MonitoredMovieCount = movies.Count(m => m.Monitored)
        };

        foreach (var movie in movies)
        {
            var file = await _movieFileRepository.FindByMovieIdAsync(movie.Id, ct);
            if (file != null)
            {
                stats.MovieFileCount++;
                stats.SizeOnDisk += file.Size;
            }
        }

        return stats;
    }

    public CollectionStatistics GetStatistics(int collectionId)
    {
        var movies = _movieRepository.GetByCollectionId(collectionId);

        var stats = new CollectionStatistics
        {
            CollectionId = collectionId,
            MovieCount = movies.Count,
            MonitoredMovieCount = movies.Count(m => m.Monitored)
        };

        foreach (var movie in movies)
        {
            var file = _movieFileRepository.FindByMovieId(movie.Id);
            if (file != null)
            {
                stats.MovieFileCount++;
                stats.SizeOnDisk += file.Size;
            }
        }

        return stats;
    }
}
