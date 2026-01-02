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

public interface ITrackRepository : IBasicRepository<Track>
{
    Task<Track?> FindByForeignIdAsync(string foreignTrackId, CancellationToken ct = default);
    Task<List<Track>> GetByAlbumIdAsync(int albumId, CancellationToken ct = default);
    Task<List<Track>> GetByArtistIdAsync(int artistId, CancellationToken ct = default);
    Task<List<Track>> GetMonitoredAsync(CancellationToken ct = default);

    Track? FindByForeignId(string foreignTrackId);
    List<Track> GetByAlbumId(int albumId);
    List<Track> GetByArtistId(int artistId);
    List<Track> GetMonitored();
}

public class TrackRepository : BasicRepository<Track>, ITrackRepository
{
    public TrackRepository(IDatabase database)
        : base(database, "MediaItems")
    {
    }

    public override async Task<IEnumerable<Track>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<Track>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Music}").ConfigureAwait(false);
    }

    public override IEnumerable<Track> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Music}");
    }

    public override async Task<IEnumerable<Track>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Music} ORDER BY \"Id\" DESC LIMIT @PageSize OFFSET @Offset",
            new { PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public override async Task<Track?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Music}",
            new { Id = id }).ConfigureAwait(false);
    }

    public override Track? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Music}",
            new { Id = id });
    }

    public async Task<Track?> FindByForeignIdAsync(string foreignTrackId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"ForeignTrackId\" = @ForeignTrackId AND \"MediaType\" = {(int)MediaType.Music}",
            new { ForeignTrackId = foreignTrackId }).ConfigureAwait(false);
    }

    public Track? FindByForeignId(string foreignTrackId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"ForeignTrackId\" = @ForeignTrackId AND \"MediaType\" = {(int)MediaType.Music}",
            new { ForeignTrackId = foreignTrackId });
    }

    public async Task<List<Track>> GetByAlbumIdAsync(int albumId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"AlbumId\" = @AlbumId AND \"MediaType\" = {(int)MediaType.Music} ORDER BY \"DiscNumber\", \"TrackNumber\"",
            new { AlbumId = albumId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetByAlbumId(int albumId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"AlbumId\" = @AlbumId AND \"MediaType\" = {(int)MediaType.Music} ORDER BY \"DiscNumber\", \"TrackNumber\"",
            new { AlbumId = albumId }).ToList();
    }

    public async Task<List<Track>> GetByArtistIdAsync(int artistId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"ArtistId\" = @ArtistId AND \"MediaType\" = {(int)MediaType.Music}",
            new { ArtistId = artistId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetByArtistId(int artistId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"ArtistId\" = @ArtistId AND \"MediaType\" = {(int)MediaType.Music}",
            new { ArtistId = artistId }).ToList();
    }

    public async Task<List<Track>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Music}",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Track> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Track>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Music}",
            new { Monitored = true }).ToList();
    }
}
