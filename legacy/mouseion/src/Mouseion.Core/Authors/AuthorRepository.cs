// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authors;

public interface IAuthorRepository : IBasicRepository<Author>
{
    Task<Author?> FindByNameAsync(string name, CancellationToken ct = default);
    Task<Author?> FindByForeignIdAsync(string foreignAuthorId, CancellationToken ct = default);
    Task<List<Author>> GetMonitoredAsync(CancellationToken ct = default);
    Task<bool> AuthorPathExistsAsync(string path, CancellationToken ct = default);

    Author? FindByName(string name);
    Author? FindByForeignId(string foreignAuthorId);
    List<Author> GetMonitored();
    bool AuthorPathExists(string path);
}

public class AuthorRepository : BasicRepository<Author>, IAuthorRepository
{
    public AuthorRepository(IDatabase database)
        : base(database)
    {
    }

    public async Task<Author?> FindByNameAsync(string name, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Author>(
            $"SELECT * FROM \"{_table}\" WHERE \"Name\" = @Name",
            new { Name = name }).ConfigureAwait(false);
    }

    public Author? FindByName(string name)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Author>(
            $"SELECT * FROM \"{_table}\" WHERE \"Name\" = @Name",
            new { Name = name });
    }

    public async Task<Author?> FindByForeignIdAsync(string foreignAuthorId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Author>(
            $"SELECT * FROM \"{_table}\" WHERE \"ForeignAuthorId\" = @ForeignAuthorId",
            new { ForeignAuthorId = foreignAuthorId }).ConfigureAwait(false);
    }

    public Author? FindByForeignId(string foreignAuthorId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Author>(
            $"SELECT * FROM \"{_table}\" WHERE \"ForeignAuthorId\" = @ForeignAuthorId",
            new { ForeignAuthorId = foreignAuthorId });
    }

    public async Task<List<Author>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Author>(
            $"SELECT * FROM \"{_table}\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Author> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Author>(
            $"SELECT * FROM \"{_table}\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ToList();
    }

    public async Task<bool> AuthorPathExistsAsync(string path, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.QuerySingleAsync<int>(
            $"SELECT COUNT(*) FROM \"{_table}\" WHERE \"Path\" = @Path",
            new { Path = path }).ConfigureAwait(false);
        return count > 0;
    }

    public bool AuthorPathExists(string path)
    {
        using var conn = _database.OpenConnection();
        var count = conn.QuerySingle<int>(
            $"SELECT COUNT(*) FROM \"{_table}\" WHERE \"Path\" = @Path",
            new { Path = path });
        return count > 0;
    }
}
