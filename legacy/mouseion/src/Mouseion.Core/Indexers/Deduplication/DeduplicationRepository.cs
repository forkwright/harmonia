// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Indexers.Deduplication;

public interface ISearchHistoryRepository : IBasicRepository<SearchHistoryEntry>
{
    Task<SearchHistoryEntry?> GetLastSearchAsync(int mediaItemId, string indexerName, CancellationToken ct = default);
    Task<List<SearchHistoryEntry>> GetByMediaItemAsync(int mediaItemId, CancellationToken ct = default);
    Task CleanupOlderThanAsync(DateTime cutoff, CancellationToken ct = default);
}

public class SearchHistoryRepository : BasicRepository<SearchHistoryEntry>, ISearchHistoryRepository
{
    public SearchHistoryRepository(IDatabase database) : base(database, "SearchHistory") { }

    public async Task<SearchHistoryEntry?> GetLastSearchAsync(int mediaItemId, string indexerName, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<SearchHistoryEntry>(
            @"SELECT * FROM ""SearchHistory"" WHERE ""MediaItemId"" = @MediaItemId AND ""IndexerName"" = @IndexerName ORDER BY ""SearchedAt"" DESC LIMIT 1",
            new { MediaItemId = mediaItemId, IndexerName = indexerName }).ConfigureAwait(false);
    }

    public async Task<List<SearchHistoryEntry>> GetByMediaItemAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<SearchHistoryEntry>(
            @"SELECT * FROM ""SearchHistory"" WHERE ""MediaItemId"" = @MediaItemId ORDER BY ""SearchedAt"" DESC",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task CleanupOlderThanAsync(DateTime cutoff, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(@"DELETE FROM ""SearchHistory"" WHERE ""SearchedAt"" < @Cutoff", new { Cutoff = cutoff }).ConfigureAwait(false);
    }
}

public interface IGrabbedReleaseRepository : IBasicRepository<GrabbedRelease>
{
    Task<bool> IsGrabbedAsync(string releaseGuid, CancellationToken ct = default);
    Task<List<GrabbedRelease>> GetByMediaItemAsync(int mediaItemId, CancellationToken ct = default);
}

public class GrabbedReleaseRepository : BasicRepository<GrabbedRelease>, IGrabbedReleaseRepository
{
    public GrabbedReleaseRepository(IDatabase database) : base(database, "GrabbedReleases") { }

    public async Task<bool> IsGrabbedAsync(string releaseGuid, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""GrabbedReleases"" WHERE ""ReleaseGuid"" = @Guid",
            new { Guid = releaseGuid }).ConfigureAwait(false) > 0;
    }

    public async Task<List<GrabbedRelease>> GetByMediaItemAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<GrabbedRelease>(
            @"SELECT * FROM ""GrabbedReleases"" WHERE ""MediaItemId"" = @MediaItemId ORDER BY ""GrabbedAt"" DESC",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return result.ToList();
    }
}

public interface ISkippedReleaseRepository : IBasicRepository<SkippedRelease>
{
    Task<bool> IsSkippedAsync(string releaseGuid, CancellationToken ct = default);
    Task<List<SkippedRelease>> GetByMediaItemAsync(int mediaItemId, CancellationToken ct = default);
}

public class SkippedReleaseRepository : BasicRepository<SkippedRelease>, ISkippedReleaseRepository
{
    public SkippedReleaseRepository(IDatabase database) : base(database, "SkippedReleases") { }

    public async Task<bool> IsSkippedAsync(string releaseGuid, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""SkippedReleases"" WHERE ""ReleaseGuid"" = @Guid",
            new { Guid = releaseGuid }).ConfigureAwait(false) > 0;
    }

    public async Task<List<SkippedRelease>> GetByMediaItemAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<SkippedRelease>(
            @"SELECT * FROM ""SkippedReleases"" WHERE ""MediaItemId"" = @MediaItemId ORDER BY ""SkippedAt"" DESC",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return result.ToList();
    }
}
