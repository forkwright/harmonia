// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.RootFolders;

public interface IRootFolderRepository : IBasicRepository<RootFolder>
{
    Task<RootFolder?> FindByPathAsync(string path, CancellationToken ct = default);
    Task<List<RootFolder>> GetByMediaTypeAsync(MediaType mediaType, CancellationToken ct = default);
    Task<bool> PathExistsAsync(string path, CancellationToken ct = default);

    RootFolder? FindByPath(string path);
    List<RootFolder> GetByMediaType(MediaType mediaType);
    bool PathExists(string path);
}

public class RootFolderRepository : BasicRepository<RootFolder>, IRootFolderRepository
{
    public RootFolderRepository(IDatabase database)
        : base(database, "RootFolders")
    {
    }

    public async Task<RootFolder?> FindByPathAsync(string path, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<RootFolder>(
            "SELECT * FROM \"RootFolders\" WHERE \"Path\" = @Path",
            new { Path = path }).ConfigureAwait(false);
    }

    public RootFolder? FindByPath(string path)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<RootFolder>(
            "SELECT * FROM \"RootFolders\" WHERE \"Path\" = @Path",
            new { Path = path });
    }

    public async Task<List<RootFolder>> GetByMediaTypeAsync(MediaType mediaType, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var results = await conn.QueryAsync<RootFolder>(
            "SELECT * FROM \"RootFolders\" WHERE \"MediaType\" = @MediaType ORDER BY \"Path\"",
            new { MediaType = (int)mediaType }).ConfigureAwait(false);
        return results.ToList();
    }

    public List<RootFolder> GetByMediaType(MediaType mediaType)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<RootFolder>(
            "SELECT * FROM \"RootFolders\" WHERE \"MediaType\" = @MediaType ORDER BY \"Path\"",
            new { MediaType = (int)mediaType }).ToList();
    }

    public async Task<bool> PathExistsAsync(string path, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"RootFolders\" WHERE \"Path\" = @Path",
            new { Path = path }).ConfigureAwait(false);
        return count > 0;
    }

    public bool PathExists(string path)
    {
        using var conn = _database.OpenConnection();
        var count = conn.ExecuteScalar<int>(
            "SELECT COUNT(*) FROM \"RootFolders\" WHERE \"Path\" = @Path",
            new { Path = path });
        return count > 0;
    }
}
