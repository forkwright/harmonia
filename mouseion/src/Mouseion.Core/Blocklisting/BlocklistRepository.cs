// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Blocklisting;

public class BlocklistRepository : BasicRepository<Blocklist>, IBlocklistRepository
{
    public BlocklistRepository(IDatabase database)
        : base(database, "Blocklists")
    {
    }

    public async Task<List<Blocklist>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Blocklist>(
            "SELECT * FROM \"Blocklists\" WHERE \"MediaItemId\" = @MediaItemId ORDER BY \"Date\" DESC",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Blocklist> GetByMediaItemId(int mediaItemId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Blocklist>(
            "SELECT * FROM \"Blocklists\" WHERE \"MediaItemId\" = @MediaItemId ORDER BY \"Date\" DESC",
            new { MediaItemId = mediaItemId }).ToList();
    }

    public async Task<List<Blocklist>> GetBySourceTitleAsync(int mediaItemId, string sourceTitle, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Blocklist>(
            "SELECT * FROM \"Blocklists\" WHERE \"MediaItemId\" = @MediaItemId AND \"SourceTitle\" LIKE @SourceTitle",
            new { MediaItemId = mediaItemId, SourceTitle = $"%{sourceTitle}%" }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Blocklist> GetBySourceTitle(int mediaItemId, string sourceTitle)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Blocklist>(
            "SELECT * FROM \"Blocklists\" WHERE \"MediaItemId\" = @MediaItemId AND \"SourceTitle\" LIKE @SourceTitle",
            new { MediaItemId = mediaItemId, SourceTitle = $"%{sourceTitle}%" }).ToList();
    }

    public async Task<List<Blocklist>> GetByTorrentInfoHashAsync(int mediaItemId, string torrentInfoHash, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Blocklist>(
            "SELECT * FROM \"Blocklists\" WHERE \"MediaItemId\" = @MediaItemId AND \"TorrentInfoHash\" LIKE @TorrentInfoHash",
            new { MediaItemId = mediaItemId, TorrentInfoHash = $"%{torrentInfoHash}%" }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Blocklist> GetByTorrentInfoHash(int mediaItemId, string torrentInfoHash)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Blocklist>(
            "SELECT * FROM \"Blocklists\" WHERE \"MediaItemId\" = @MediaItemId AND \"TorrentInfoHash\" LIKE @TorrentInfoHash",
            new { MediaItemId = mediaItemId, TorrentInfoHash = $"%{torrentInfoHash}%" }).ToList();
    }

    public async Task DeleteByMediaItemIdsAsync(List<int> mediaItemIds, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "DELETE FROM \"Blocklists\" WHERE \"MediaItemId\" IN @MediaItemIds",
            new { MediaItemIds = mediaItemIds }).ConfigureAwait(false);
    }

    public void DeleteByMediaItemIds(List<int> mediaItemIds)
    {
        using var conn = _database.OpenConnection();
        conn.Execute(
            "DELETE FROM \"Blocklists\" WHERE \"MediaItemId\" IN @MediaItemIds",
            new { MediaItemIds = mediaItemIds });
    }
}
