// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Progress;

public interface IMediaProgressRepository : IBasicRepository<MediaProgress>
{
    Task<MediaProgress?> GetByMediaItemIdAsync(int mediaItemId, string userId = "default", CancellationToken ct = default);
    Task<List<MediaProgress>> GetInProgressAsync(string userId = "default", int limit = 20, CancellationToken ct = default);
    Task<List<MediaProgress>> GetRecentlyPlayedAsync(string userId = "default", int limit = 50, CancellationToken ct = default);
    Task UpsertAsync(MediaProgress progress, CancellationToken ct = default);
    Task DeleteByMediaItemIdAsync(int mediaItemId, string userId = "default", CancellationToken ct = default);
}

public class MediaProgressRepository : BasicRepository<MediaProgress>, IMediaProgressRepository
{
    public MediaProgressRepository(IDatabase database)
        : base(database, "MediaProgress")
    {
    }

    public async Task<MediaProgress?> GetByMediaItemIdAsync(int mediaItemId, string userId = "default", CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MediaProgress>(
            "SELECT * FROM \"MediaProgress\" WHERE \"MediaItemId\" = @MediaItemId AND \"UserId\" = @UserId",
            new { MediaItemId = mediaItemId, UserId = userId }).ConfigureAwait(false);
    }

    public async Task<List<MediaProgress>> GetInProgressAsync(string userId = "default", int limit = 20, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MediaProgress>(
            @"SELECT * FROM ""MediaProgress""
              WHERE ""UserId"" = @UserId AND ""IsComplete"" = 0
              ORDER BY ""LastPlayedAt"" DESC
              LIMIT @Limit",
            new { UserId = userId, Limit = limit }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<MediaProgress>> GetRecentlyPlayedAsync(string userId = "default", int limit = 50, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MediaProgress>(
            @"SELECT * FROM ""MediaProgress""
              WHERE ""UserId"" = @UserId
              ORDER BY ""LastPlayedAt"" DESC
              LIMIT @Limit",
            new { UserId = userId, Limit = limit }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task UpsertAsync(MediaProgress progress, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var existing = await GetByMediaItemIdAsync(progress.MediaItemId, progress.UserId, ct).ConfigureAwait(false);

        if (existing == null)
        {
            progress.CreatedAt = DateTime.UtcNow;
            progress.UpdatedAt = DateTime.UtcNow;
            await InsertAsync(progress, ct).ConfigureAwait(false);
        }
        else
        {
            progress.Id = existing.Id;
            progress.CreatedAt = existing.CreatedAt;
            progress.UpdatedAt = DateTime.UtcNow;
            await UpdateAsync(progress, ct).ConfigureAwait(false);
        }
    }

    public async Task DeleteByMediaItemIdAsync(int mediaItemId, string userId = "default", CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "DELETE FROM \"MediaProgress\" WHERE \"MediaItemId\" = @MediaItemId AND \"UserId\" = @UserId",
            new { MediaItemId = mediaItemId, UserId = userId }).ConfigureAwait(false);
    }
}
