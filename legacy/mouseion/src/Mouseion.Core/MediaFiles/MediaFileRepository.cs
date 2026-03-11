// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.MediaFiles;

// Repository for MediaFile with custom queries
public interface IMediaFileRepository : IBasicRepository<MediaFile>
{
    Task<List<MediaFile>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task<MediaFile?> FindByPathAsync(string path, CancellationToken ct = default);

    IEnumerable<MediaFile> GetByMediaItemId(int mediaItemId);
    MediaFile? FindByPath(string path);
}

public class MediaFileRepository : BasicRepository<MediaFile>, IMediaFileRepository
{
    public MediaFileRepository(IDatabase database)
        : base(database)
    {
    }

    public async Task<List<MediaFile>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var results = await conn.QueryAsync<MediaFile>(
            $"SELECT * FROM \"{_table}\" WHERE \"MediaItemId\" = @MediaItemId",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return results.ToList();
    }

    public async Task<MediaFile?> FindByPathAsync(string path, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleOrDefaultAsync<MediaFile>(
            $"SELECT * FROM \"{_table}\" WHERE \"Path\" = @Path",
            new { Path = path }).ConfigureAwait(false);
    }

    public IEnumerable<MediaFile> GetByMediaItemId(int mediaItemId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MediaFile>(
            $"SELECT * FROM \"{_table}\" WHERE \"MediaItemId\" = @MediaItemId",
            new { MediaItemId = mediaItemId });
    }

    public MediaFile? FindByPath(string path)
    {
        using var conn = _database.OpenConnection();
        return conn.QuerySingleOrDefault<MediaFile>(
            $"SELECT * FROM \"{_table}\" WHERE \"Path\" = @Path",
            new { Path = path });
    }
}
