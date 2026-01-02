// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Music;

public interface IMusicFileRepository : IBasicRepository<MusicFile>
{
    Task<List<MusicFile>> GetByTrackIdAsync(int trackId, CancellationToken ct = default);
    Task<List<MusicFile>> GetByAlbumIdAsync(int albumId, CancellationToken ct = default);
    Task<MusicFile?> FindByPathAsync(string relativePath, CancellationToken ct = default);

    List<MusicFile> GetByTrackId(int trackId);
    List<MusicFile> GetByAlbumId(int albumId);
    MusicFile? FindByPath(string relativePath);
}

public class MusicFileRepository : BasicRepository<MusicFile>, IMusicFileRepository
{
    public MusicFileRepository(IDatabase database)
        : base(database, "MusicFiles")
    {
    }

    public async Task<List<MusicFile>> GetByTrackIdAsync(int trackId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MusicFile>(
            "SELECT * FROM \"MusicFiles\" WHERE \"TrackId\" = @TrackId",
            new { TrackId = trackId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MusicFile> GetByTrackId(int trackId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MusicFile>(
            "SELECT * FROM \"MusicFiles\" WHERE \"TrackId\" = @TrackId",
            new { TrackId = trackId }).ToList();
    }

    public async Task<List<MusicFile>> GetByAlbumIdAsync(int albumId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MusicFile>(
            "SELECT * FROM \"MusicFiles\" WHERE \"AlbumId\" = @AlbumId",
            new { AlbumId = albumId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MusicFile> GetByAlbumId(int albumId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MusicFile>(
            "SELECT * FROM \"MusicFiles\" WHERE \"AlbumId\" = @AlbumId",
            new { AlbumId = albumId }).ToList();
    }

    public async Task<MusicFile?> FindByPathAsync(string relativePath, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MusicFile>(
            "SELECT * FROM \"MusicFiles\" WHERE \"RelativePath\" = @RelativePath",
            new { RelativePath = relativePath }).ConfigureAwait(false);
    }

    public MusicFile? FindByPath(string relativePath)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<MusicFile>(
            "SELECT * FROM \"MusicFiles\" WHERE \"RelativePath\" = @RelativePath",
            new { RelativePath = relativePath });
    }
}
