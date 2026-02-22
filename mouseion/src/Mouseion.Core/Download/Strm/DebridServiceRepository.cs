// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Download.Strm;

public interface IDebridServiceRepository : IBasicRepository<DebridServiceDefinition>
{
    List<DebridServiceDefinition> GetEnabled();
    DebridServiceDefinition? GetByProvider(DebridProvider provider);
}

public class DebridServiceRepository : BasicRepository<DebridServiceDefinition>, IDebridServiceRepository
{
    public DebridServiceRepository(IDatabase database) : base(database, "DebridServices") { }

    public List<DebridServiceDefinition> GetEnabled()
    {
        return All().Where(d => d.Enabled).OrderBy(d => d.Priority).ToList();
    }

    public DebridServiceDefinition? GetByProvider(DebridProvider provider)
    {
        return All().FirstOrDefault(d => d.Provider == provider && d.Enabled);
    }
}

public interface IStrmFileRepository : IBasicRepository<StrmFile>
{
    Task<StrmFile?> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task<List<StrmFile>> GetExpiredAsync(CancellationToken ct = default);
    Task<List<StrmFile>> GetInvalidAsync(CancellationToken ct = default);
    Task<int> CountValidAsync(CancellationToken ct = default);
}

public class StrmFileRepository : BasicRepository<StrmFile>, IStrmFileRepository
{
    private new readonly IDatabase _database;

    public StrmFileRepository(IDatabase database) : base(database, "StrmFiles")
    {
        _database = database;
    }

    public async Task<StrmFile?> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<StrmFile>(
            @"SELECT * FROM ""StrmFiles"" WHERE ""MediaItemId"" = @MediaItemId AND ""IsValid"" = 1 ORDER BY ""CreatedAt"" DESC LIMIT 1",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
    }

    public async Task<List<StrmFile>> GetExpiredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<StrmFile>(
            @"SELECT * FROM ""StrmFiles"" WHERE ""IsValid"" = 1 AND ""ExpiresAt"" IS NOT NULL AND ""ExpiresAt"" < @Now",
            new { Now = DateTime.UtcNow }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<StrmFile>> GetInvalidAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<StrmFile>(
            @"SELECT * FROM ""StrmFiles"" WHERE ""IsValid"" = 0").ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> CountValidAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""StrmFiles"" WHERE ""IsValid"" = 1").ConfigureAwait(false);
    }
}
