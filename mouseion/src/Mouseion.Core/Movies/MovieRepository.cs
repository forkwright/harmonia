// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Movies;

public interface IMovieRepository : IBasicRepository<Movie>
{
    Task<Movie?> FindByTmdbIdAsync(string tmdbId, CancellationToken ct = default);
    Task<Movie?> FindByImdbIdAsync(string imdbId, CancellationToken ct = default);
    Task<List<Movie>> GetByCollectionIdAsync(int collectionId, CancellationToken ct = default);
    Task<List<Movie>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<Movie>> GetMoviesBetweenDatesAsync(DateTime start, DateTime end, bool includeUnmonitored = false, CancellationToken ct = default);

    Movie? FindByTmdbId(string tmdbId);
    Movie? FindByImdbId(string imdbId);
    List<Movie> GetByCollectionId(int collectionId);
    List<Movie> GetMonitored();
    List<Movie> GetMoviesBetweenDates(DateTime start, DateTime end, bool includeUnmonitored = false);
}

public class MovieRepository : BasicRepository<Movie>, IMovieRepository
{
    public MovieRepository(IDatabase database)
        : base(database, "MediaItems")
    {
    }

    public override async Task<IEnumerable<Movie>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<Movie>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Movie}").ConfigureAwait(false);
    }

    public override IEnumerable<Movie> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Movie>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Movie}");
    }

    public override async Task<IEnumerable<Movie>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Movie} ORDER BY \"Title\", \"Year\" DESC LIMIT @PageSize OFFSET @Offset",
            new { PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public override async Task<Movie?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Movie}",
            new { Id = id }).ConfigureAwait(false);
    }

    public override Movie? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Movie}",
            new { Id = id });
    }

    public async Task<Movie?> FindByTmdbIdAsync(string tmdbId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"TmdbId\" = @TmdbId AND \"MediaType\" = {(int)MediaType.Movie}",
            new { TmdbId = tmdbId }).ConfigureAwait(false);
    }

    public Movie? FindByTmdbId(string tmdbId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"TmdbId\" = @TmdbId AND \"MediaType\" = {(int)MediaType.Movie}",
            new { TmdbId = tmdbId });
    }

    public async Task<Movie?> FindByImdbIdAsync(string imdbId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"ImdbId\" = @ImdbId AND \"MediaType\" = {(int)MediaType.Movie}",
            new { ImdbId = imdbId }).ConfigureAwait(false);
    }

    public Movie? FindByImdbId(string imdbId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"ImdbId\" = @ImdbId AND \"MediaType\" = {(int)MediaType.Movie}",
            new { ImdbId = imdbId });
    }

    public async Task<List<Movie>> GetByCollectionIdAsync(int collectionId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"CollectionId\" = @CollectionId AND \"MediaType\" = {(int)MediaType.Movie} ORDER BY \"Year\"",
            new { CollectionId = collectionId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Movie> GetByCollectionId(int collectionId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"CollectionId\" = @CollectionId AND \"MediaType\" = {(int)MediaType.Movie} ORDER BY \"Year\"",
            new { CollectionId = collectionId }).ToList();
    }

    public async Task<List<Movie>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Movie}",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Movie> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Movie>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Movie}",
            new { Monitored = true }).ToList();
    }

    public async Task<List<Movie>> GetMoviesBetweenDatesAsync(DateTime start, DateTime end, bool includeUnmonitored = false, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var sql = includeUnmonitored
            ? $@"SELECT * FROM ""MediaItems""
                 WHERE ""MediaType"" = {(int)MediaType.Movie}
                 AND (
                     (""InCinemas"" IS NOT NULL AND ""InCinemas"" >= @Start AND ""InCinemas"" <= @End)
                     OR (""DigitalRelease"" IS NOT NULL AND ""DigitalRelease"" >= @Start AND ""DigitalRelease"" <= @End)
                     OR (""PhysicalRelease"" IS NOT NULL AND ""PhysicalRelease"" >= @Start AND ""PhysicalRelease"" <= @End)
                 )"
            : $@"SELECT * FROM ""MediaItems""
                 WHERE ""MediaType"" = {(int)MediaType.Movie}
                 AND ""Monitored"" = @Monitored
                 AND (
                     (""InCinemas"" IS NOT NULL AND ""InCinemas"" >= @Start AND ""InCinemas"" <= @End)
                     OR (""DigitalRelease"" IS NOT NULL AND ""DigitalRelease"" >= @Start AND ""DigitalRelease"" <= @End)
                     OR (""PhysicalRelease"" IS NOT NULL AND ""PhysicalRelease"" >= @Start AND ""PhysicalRelease"" <= @End)
                 )";

        var result = await conn.QueryAsync<Movie>(sql, new { Start = start, End = end, Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Movie> GetMoviesBetweenDates(DateTime start, DateTime end, bool includeUnmonitored = false)
    {
        using var conn = _database.OpenConnection();

        var sql = includeUnmonitored
            ? $@"SELECT * FROM ""MediaItems""
                 WHERE ""MediaType"" = {(int)MediaType.Movie}
                 AND (
                     (""InCinemas"" IS NOT NULL AND ""InCinemas"" >= @Start AND ""InCinemas"" <= @End)
                     OR (""DigitalRelease"" IS NOT NULL AND ""DigitalRelease"" >= @Start AND ""DigitalRelease"" <= @End)
                     OR (""PhysicalRelease"" IS NOT NULL AND ""PhysicalRelease"" >= @Start AND ""PhysicalRelease"" <= @End)
                 )"
            : $@"SELECT * FROM ""MediaItems""
                 WHERE ""MediaType"" = {(int)MediaType.Movie}
                 AND ""Monitored"" = @Monitored
                 AND (
                     (""InCinemas"" IS NOT NULL AND ""InCinemas"" >= @Start AND ""InCinemas"" <= @End)
                     OR (""DigitalRelease"" IS NOT NULL AND ""DigitalRelease"" >= @Start AND ""DigitalRelease"" <= @End)
                     OR (""PhysicalRelease"" IS NOT NULL AND ""PhysicalRelease"" >= @Start AND ""PhysicalRelease"" <= @End)
                 )";

        return conn.Query<Movie>(sql, new { Start = start, End = end, Monitored = true }).ToList();
    }
}
