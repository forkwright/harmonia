// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Movies;

public interface IMovieFileRepository : IBasicRepository<MovieFile>
{
    Task<MovieFile?> FindByMovieIdAsync(int movieId, CancellationToken ct = default);
    Task<List<MovieFile>> GetByPathAsync(string path, CancellationToken ct = default);

    MovieFile? FindByMovieId(int movieId);
    List<MovieFile> GetByPath(string path);
}

public class MovieFileRepository : BasicRepository<MovieFile>, IMovieFileRepository
{
    public MovieFileRepository(IDatabase database)
        : base(database, "MovieFiles")
    {
    }

    public async Task<MovieFile?> FindByMovieIdAsync(int movieId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MovieFile>(
            "SELECT * FROM \"MovieFiles\" WHERE \"MovieId\" = @MovieId",
            new { MovieId = movieId }).ConfigureAwait(false);
    }

    public MovieFile? FindByMovieId(int movieId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<MovieFile>(
            "SELECT * FROM \"MovieFiles\" WHERE \"MovieId\" = @MovieId",
            new { MovieId = movieId });
    }

    public async Task<List<MovieFile>> GetByPathAsync(string path, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MovieFile>(
            "SELECT * FROM \"MovieFiles\" WHERE \"Path\" LIKE @Path",
            new { Path = $"{path}%" }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MovieFile> GetByPath(string path)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MovieFile>(
            "SELECT * FROM \"MovieFiles\" WHERE \"Path\" LIKE @Path",
            new { Path = $"{path}%" }).ToList();
    }
}
