// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.Filtering;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Music;

public interface ITrackRepository : IBasicRepository<Track>
{
    Task<Track?> FindByForeignIdAsync(string foreignTrackId, CancellationToken ct = default);
    Task<List<Track>> GetByAlbumIdAsync(int albumId, CancellationToken ct = default);
    Task<List<Track>> GetByArtistIdAsync(int artistId, CancellationToken ct = default);
    Task<List<Track>> GetByIdsAsync(IEnumerable<int> ids, CancellationToken ct = default);
    Task<List<Track>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<Track>> FilterAsync(FilterRequest request, CancellationToken ct = default);

    Track? FindByForeignId(string foreignTrackId);
    List<Track> GetByAlbumId(int albumId);
    List<Track> GetByArtistId(int artistId);
    List<Track> GetByIds(IEnumerable<int> ids);
    List<Track> GetMonitored();
    List<Track> Filter(FilterRequest request);
}

public class TrackRepository : BasicRepository<Track>, ITrackRepository
{
    private readonly IFilterQueryBuilder _queryBuilder;

    public TrackRepository(IDatabase database, IFilterQueryBuilder queryBuilder)
        : base(database, "MediaItems")
    {
        _queryBuilder = queryBuilder;
    }

    public override async Task<IEnumerable<Track>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<Track>("SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType", new { MediaType = (int)MediaType.Music }).ConfigureAwait(false);
    }

    public override IEnumerable<Track> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>("SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType", new { MediaType = (int)MediaType.Music });
    }

    public override async Task<IEnumerable<Track>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType ORDER BY \"Id\" DESC LIMIT @PageSize OFFSET @Offset",
            new { MediaType = (int)MediaType.Music, PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public override async Task<Track?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = @MediaType",
            new { Id = id, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
    }

    public override Track? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = @MediaType",
            new { Id = id, MediaType = (int)MediaType.Music });
    }

    public async Task<Track?> FindByForeignIdAsync(string foreignTrackId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"ForeignTrackId\" = @ForeignTrackId AND \"MediaType\" = @MediaType",
            new { ForeignTrackId = foreignTrackId, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
    }

    public Track? FindByForeignId(string foreignTrackId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"ForeignTrackId\" = @ForeignTrackId AND \"MediaType\" = @MediaType",
            new { ForeignTrackId = foreignTrackId, MediaType = (int)MediaType.Music });
    }

    public async Task<List<Track>> GetByAlbumIdAsync(int albumId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"AlbumId\" = @AlbumId AND \"MediaType\" = @MediaType ORDER BY \"DiscNumber\", \"TrackNumber\"",
            new { AlbumId = albumId, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetByAlbumId(int albumId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"AlbumId\" = @AlbumId AND \"MediaType\" = @MediaType ORDER BY \"DiscNumber\", \"TrackNumber\"",
            new { AlbumId = albumId, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<List<Track>> GetByArtistIdAsync(int artistId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"ArtistId\" = @ArtistId AND \"MediaType\" = @MediaType",
            new { ArtistId = artistId, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetByArtistId(int artistId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"ArtistId\" = @ArtistId AND \"MediaType\" = @MediaType",
            new { ArtistId = artistId, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<List<Track>> GetByIdsAsync(IEnumerable<int> ids, CancellationToken ct = default)
    {
        var idList = ids.ToList();
        if (idList.Count == 0)
        {
            return new List<Track>();
        }

        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"Id\" IN @Ids AND \"MediaType\" = @MediaType",
            new { Ids = idList, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetByIds(IEnumerable<int> ids)
    {
        var idList = ids.ToList();
        if (idList.Count == 0)
        {
            return new List<Track>();
        }

        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"Id\" IN @Ids AND \"MediaType\" = @MediaType",
            new { Ids = idList, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<List<Track>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = @MediaType",
            new { Monitored = true, MediaType = (int)MediaType.Music }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            "SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = @MediaType",
            new { Monitored = true, MediaType = (int)MediaType.Music }).ToList();
    }

    public async Task<List<Track>> FilterAsync(FilterRequest request, CancellationToken ct = default)
    {
        var (sql, parameters) = _queryBuilder.BuildQuery(request, "MusicFiles");

        var joinSql = sql.Replace(
            "SELECT * FROM \"MusicFiles\"",
            @"SELECT m.* FROM ""MediaItems"" m
               INNER JOIN ""MusicFiles"" mf ON m.""Id"" = mf.""TrackId""
               LEFT JOIN ""Albums"" al ON m.""AlbumId"" = al.""Id""
               LEFT JOIN ""Artists"" ar ON m.""ArtistId"" = ar.""Id""
               WHERE m.""MediaType"" = @MediaType AND");

        // Add MediaType parameter to the existing parameters
        parameters.Add("MediaType", (int)MediaType.Music);

        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(joinSql, parameters).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> Filter(FilterRequest request)
    {
        var (sql, parameters) = _queryBuilder.BuildQuery(request, "MusicFiles");

        var joinSql = sql.Replace(
            "SELECT * FROM \"MusicFiles\"",
            @"SELECT m.* FROM ""MediaItems"" m
               INNER JOIN ""MusicFiles"" mf ON m.""Id"" = mf.""TrackId""
               LEFT JOIN ""Albums"" al ON m.""AlbumId"" = al.""Id""
               LEFT JOIN ""Artists"" ar ON m.""ArtistId"" = ar.""Id""
               WHERE m.""MediaType"" = @MediaType AND");

        // Add MediaType parameter to the existing parameters
        parameters.Add("MediaType", (int)MediaType.Music);

        using var conn = _database.OpenConnection();
        return conn.Query<Track>(joinSql, parameters).ToList();
    }
}
