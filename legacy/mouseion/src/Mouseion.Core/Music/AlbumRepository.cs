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

namespace Mouseion.Core.Music;

public class AlbumStats
{
    public int AlbumId { get; set; }
    public int TrackCount { get; set; }
    public int DurationSeconds { get; set; }
}

public interface IAlbumRepository : IBasicRepository<Album>
{
    Task<Album?> FindByTitleAsync(string title, int? artistId, CancellationToken ct = default);
    Task<Album?> FindByForeignIdAsync(string foreignAlbumId, CancellationToken ct = default);
    Task<List<Album>> GetByArtistIdAsync(int artistId, CancellationToken ct = default);
    Task<List<Album>> GetMonitoredAsync(CancellationToken ct = default);
    Task<bool> AlbumExistsAsync(int artistId, string title, CancellationToken ct = default);
    Task<List<Album>> GetVersionsAsync(string releaseGroupMbid, CancellationToken ct = default);
    Task<Dictionary<int, AlbumStats>> GetBatchStatsAsync(IEnumerable<int> albumIds, CancellationToken ct = default);

    Album? FindByTitle(string title, int? artistId);
    Album? FindByForeignId(string foreignAlbumId);
    List<Album> GetByArtistId(int artistId);
    List<Album> GetMonitored();
    bool AlbumExists(int artistId, string title);
    List<Album> GetVersions(string releaseGroupMbid);
}

public class AlbumRepository : BasicRepository<Album>, IAlbumRepository
{
    public AlbumRepository(IDatabase database)
        : base(database, "Albums")
    {
    }

    public override async Task<IEnumerable<Album>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<Album>("SELECT * FROM \"Albums\" WHERE \"MediaType\" = @MediaType", new { MediaType = (int)MediaType.Music }).ConfigureAwait(false);
    }

    public override IEnumerable<Album> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Album>("SELECT * FROM \"Albums\" WHERE \"MediaType\" = @MediaType", new { MediaType = (int)MediaType.Music });
    }

    public override async Task<IEnumerable<Album>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<Album>(
            "SELECT * FROM \"Albums\" WHERE \"MediaType\" = @MediaType ORDER BY \"Id\" DESC LIMIT @PageSize OFFSET @Offset",
            new { MediaType = (int)MediaType.Music, PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public async Task<Album?> FindByTitleAsync(string title, int? artistId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        if (artistId.HasValue)
        {
            return await conn.QueryFirstOrDefaultAsync<Album>(
                "SELECT * FROM \"Albums\" WHERE \"Title\" = @Title AND \"ArtistId\" = @ArtistId AND \"MediaType\" = @MediaType",
                new { Title = title, ArtistId = artistId.Value, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        }
        else
        {
            return await conn.QueryFirstOrDefaultAsync<Album>(
                "SELECT * FROM \"Albums\" WHERE \"Title\" = @Title AND \"MediaType\" = @MediaType",
                new { Title = title, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        }
    }

    public Album? FindByTitle(string title, int? artistId)
    {
        using var conn = _database.OpenConnection();
        if (artistId.HasValue)
        {
            return conn.QueryFirstOrDefault<Album>(
                "SELECT * FROM \"Albums\" WHERE \"Title\" = @Title AND \"ArtistId\" = @ArtistId AND \"MediaType\" = @MediaType",
                new { Title = title, ArtistId = artistId.Value, MediaType = (int)MediaType.Music });
        }
        else
        {
            return conn.QueryFirstOrDefault<Album>(
                "SELECT * FROM \"Albums\" WHERE \"Title\" = @Title AND \"MediaType\" = @MediaType",
                new { Title = title, MediaType = (int)MediaType.Music });
        }
    }

    public async Task<Album?> FindByForeignIdAsync(string foreignAlbumId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Album>(
            "SELECT * FROM \"Albums\" WHERE \"ForeignAlbumId\" = @ForeignAlbumId AND \"MediaType\" = @MediaType",
            new { ForeignAlbumId = foreignAlbumId, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
    }

    public Album? FindByForeignId(string foreignAlbumId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Album>(
            "SELECT * FROM \"Albums\" WHERE \"ForeignAlbumId\" = @ForeignAlbumId AND \"MediaType\" = @MediaType",
            new { ForeignAlbumId = foreignAlbumId, MediaType = (int)MediaType.Music });
    }

    public async Task<List<Album>> GetByArtistIdAsync(int artistId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Album>(
            "SELECT * FROM \"Albums\" WHERE \"ArtistId\" = @ArtistId AND \"MediaType\" = @MediaType",
            new { ArtistId = artistId, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Album> GetByArtistId(int artistId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Album>(
            "SELECT * FROM \"Albums\" WHERE \"ArtistId\" = @ArtistId AND \"MediaType\" = @MediaType",
            new { ArtistId = artistId, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<List<Album>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Album>(
            "SELECT * FROM \"Albums\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = @MediaType",
            new { Monitored = true, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Album> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Album>(
            "SELECT * FROM \"Albums\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = @MediaType",
            new { Monitored = true, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<bool> AlbumExistsAsync(int artistId, string title, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM \"Albums\" WHERE \"ArtistId\" = @ArtistId AND \"Title\" = @Title AND \"MediaType\" = @MediaType",
            new { ArtistId = artistId, Title = title, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return count > 0;
    }

    public bool AlbumExists(int artistId, string title)
    {
        using var conn = _database.OpenConnection();
        var count = conn.QuerySingle<int>(
            "SELECT COUNT(*) FROM \"Albums\" WHERE \"ArtistId\" = @ArtistId AND \"Title\" = @Title AND \"MediaType\" = @MediaType",
            new { ArtistId = artistId, Title = title, MediaType = (int)MediaType.Music });
        return count > 0;
    }

    public async Task<List<Album>> GetVersionsAsync(string releaseGroupMbid, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Album>(
            "SELECT * FROM \"Albums\" WHERE \"ReleaseGroupMbid\" = @ReleaseGroupMbid AND \"MediaType\" = @MediaType ORDER BY \"ReleaseDate\" DESC",
            new { ReleaseGroupMbid = releaseGroupMbid, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Album> GetVersions(string releaseGroupMbid)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Album>(
            "SELECT * FROM \"Albums\" WHERE \"ReleaseGroupMbid\" = @ReleaseGroupMbid AND \"MediaType\" = @MediaType ORDER BY \"ReleaseDate\" DESC",
            new { ReleaseGroupMbid = releaseGroupMbid, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<Dictionary<int, AlbumStats>> GetBatchStatsAsync(IEnumerable<int> albumIds, CancellationToken ct = default)
    {
        var ids = albumIds.ToList();
        if (ids.Count == 0) return new Dictionary<int, AlbumStats>();

        using var conn = _database.OpenConnection();
        var stats = await conn.QueryAsync<AlbumStats>(
            @"SELECT ""AlbumId"", COUNT(*) AS ""TrackCount"", COALESCE(SUM(""DurationSeconds""), 0) AS ""DurationSeconds""
              FROM ""MediaItems""
              WHERE ""AlbumId"" IN @Ids AND ""MediaType"" = @MediaType
              GROUP BY ""AlbumId""",
            new { Ids = ids, MediaType = (int)MediaType.Music }).ConfigureAwait(false);

        return stats.ToDictionary(s => s.AlbumId);
    }
}
