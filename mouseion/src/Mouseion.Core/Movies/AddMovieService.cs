// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.Movies;

public interface IAddMovieService
{
    Task<Movie> AddMovieAsync(Movie movie, CancellationToken ct = default);
    Task<List<Movie>> AddMoviesAsync(List<Movie> movies, CancellationToken ct = default);

    Movie AddMovie(Movie movie);
    List<Movie> AddMovies(List<Movie> movies);
}

public class AddMovieService : IAddMovieService
{
    private readonly IMovieRepository _movieRepository;
    private readonly ICollectionRepository _collectionRepository;
    private readonly ILogger<AddMovieService> _logger;

    public AddMovieService(
        IMovieRepository movieRepository,
        ICollectionRepository collectionRepository,
        ILogger<AddMovieService> logger)
    {
        _movieRepository = movieRepository;
        _collectionRepository = collectionRepository;
        _logger = logger;
    }

    public async Task<Movie> AddMovieAsync(Movie movie, CancellationToken ct = default)
    {
        ValidateMovie(movie);

        if (movie.CollectionId.HasValue)
        {
            var collection = await _collectionRepository.FindAsync(movie.CollectionId.Value, ct).ConfigureAwait(false);
            if (collection == null)
            {
                throw new ArgumentException($"Collection with ID {movie.CollectionId.Value} not found", nameof(movie));
            }
        }

        if (!string.IsNullOrWhiteSpace(movie.TmdbId))
        {
            var existing = await _movieRepository.FindByTmdbIdAsync(movie.TmdbId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Movie already exists: {MovieTitle} ({Year}) - TMDB ID: {TmdbId}",
                    movie.Title, movie.Year, movie.TmdbId);
                return existing;
            }
        }

        if (!string.IsNullOrWhiteSpace(movie.ImdbId))
        {
            var existing = await _movieRepository.FindByImdbIdAsync(movie.ImdbId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Movie already exists: {MovieTitle} ({Year}) - IMDB ID: {ImdbId}",
                    movie.Title, movie.Year, movie.ImdbId);
                return existing;
            }
        }

        movie.Added = DateTime.UtcNow;
        movie.Monitored = true;

        var added = await _movieRepository.InsertAsync(movie, ct).ConfigureAwait(false);
        _logger.LogInformation("Added movie: {MovieTitle} ({Year}) - TMDB ID: {TmdbId}, Collection ID: {CollectionId}",
            added.Title, added.Year, added.TmdbId, added.CollectionId);

        return added;
    }

    public Movie AddMovie(Movie movie)
    {
        ValidateMovie(movie);

        if (movie.CollectionId.HasValue)
        {
            var collection = _collectionRepository.Find(movie.CollectionId.Value);
            if (collection == null)
            {
                throw new ArgumentException($"Collection with ID {movie.CollectionId.Value} not found", nameof(movie));
            }
        }

        if (!string.IsNullOrWhiteSpace(movie.TmdbId))
        {
            var existing = _movieRepository.FindByTmdbId(movie.TmdbId);
            if (existing != null)
            {
                _logger.LogInformation("Movie already exists: {MovieTitle} ({Year}) - TMDB ID: {TmdbId}",
                    movie.Title, movie.Year, movie.TmdbId);
                return existing;
            }
        }

        if (!string.IsNullOrWhiteSpace(movie.ImdbId))
        {
            var existing = _movieRepository.FindByImdbId(movie.ImdbId);
            if (existing != null)
            {
                _logger.LogInformation("Movie already exists: {MovieTitle} ({Year}) - IMDB ID: {ImdbId}",
                    movie.Title, movie.Year, movie.ImdbId);
                return existing;
            }
        }

        movie.Added = DateTime.UtcNow;
        movie.Monitored = true;

        var added = _movieRepository.Insert(movie);
        _logger.LogInformation("Added movie: {MovieTitle} ({Year}) - TMDB ID: {TmdbId}, Collection ID: {CollectionId}",
            added.Title, added.Year, added.TmdbId, added.CollectionId);

        return added;
    }

    public async Task<List<Movie>> AddMoviesAsync(List<Movie> movies, CancellationToken ct = default)
    {
        var addedMovies = new List<Movie>();

        foreach (var movie in movies)
        {
            try
            {
                var added = await AddMovieAsync(movie, ct).ConfigureAwait(false);
                addedMovies.Add(added);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error adding movie: {MovieTitle} ({Year})", movie.Title, movie.Year);
            }
        }

        return addedMovies;
    }

    public List<Movie> AddMovies(List<Movie> movies)
    {
        var addedMovies = new List<Movie>();

        foreach (var movie in movies)
        {
            try
            {
                var added = AddMovie(movie);
                addedMovies.Add(added);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error adding movie: {MovieTitle} ({Year})", movie.Title, movie.Year);
            }
        }

        return addedMovies;
    }

    private void ValidateMovie(Movie movie)
    {
        if (string.IsNullOrWhiteSpace(movie.Title))
        {
            throw new ArgumentException("Movie title is required", nameof(movie));
        }

        if (movie.Year <= 0)
        {
            throw new ArgumentException("Movie year must be greater than 0", nameof(movie));
        }

        if (movie.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(movie));
        }
    }
}
