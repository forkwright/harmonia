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

public interface IPlaybackSessionRepository : IBasicRepository<PlaybackSession>
{
    Task<PlaybackSession?> GetBySessionIdAsync(string sessionId, CancellationToken ct = default);
    Task<List<PlaybackSession>> GetActiveSessionsAsync(string userId = "default", CancellationToken ct = default);
    Task<List<PlaybackSession>> GetRecentSessionsAsync(string userId = "default", int limit = 100, CancellationToken ct = default);
    Task<List<PlaybackSession>> GetByMediaItemIdAsync(int mediaItemId, string userId = "default", CancellationToken ct = default);
    Task EndSessionAsync(string sessionId, long endPositionMs, CancellationToken ct = default);
}

public class PlaybackSessionRepository : BasicRepository<PlaybackSession>, IPlaybackSessionRepository
{
    public PlaybackSessionRepository(IDatabase database)
        : base(database, "PlaybackSessions")
    {
    }

    public async Task<PlaybackSession?> GetBySessionIdAsync(string sessionId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<PlaybackSession>(
            "SELECT * FROM \"PlaybackSessions\" WHERE \"SessionId\" = @SessionId",
            new { SessionId = sessionId }).ConfigureAwait(false);
    }

    public async Task<List<PlaybackSession>> GetActiveSessionsAsync(string userId = "default", CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PlaybackSession>(
            @"SELECT * FROM ""PlaybackSessions""
              WHERE ""UserId"" = @UserId AND ""IsActive"" = 1
              ORDER BY ""StartedAt"" DESC",
            new { UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<PlaybackSession>> GetRecentSessionsAsync(string userId = "default", int limit = 100, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PlaybackSession>(
            @"SELECT * FROM ""PlaybackSessions""
              WHERE ""UserId"" = @UserId
              ORDER BY ""StartedAt"" DESC
              LIMIT @Limit",
            new { UserId = userId, Limit = limit }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<PlaybackSession>> GetByMediaItemIdAsync(int mediaItemId, string userId = "default", CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PlaybackSession>(
            @"SELECT * FROM ""PlaybackSessions""
              WHERE ""MediaItemId"" = @MediaItemId AND ""UserId"" = @UserId
              ORDER BY ""StartedAt"" DESC",
            new { MediaItemId = mediaItemId, UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task EndSessionAsync(string sessionId, long endPositionMs, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"UPDATE ""PlaybackSessions""
              SET ""EndedAt"" = @EndedAt,
                  ""EndPositionMs"" = @EndPositionMs,
                  ""IsActive"" = 0,
                  ""DurationMs"" = @DurationMs
              WHERE ""SessionId"" = @SessionId",
            new
            {
                SessionId = sessionId,
                EndedAt = DateTime.UtcNow,
                EndPositionMs = endPositionMs,
                DurationMs = (long)(DateTime.UtcNow - (await GetBySessionIdAsync(sessionId, ct).ConfigureAwait(false))!.StartedAt).TotalMilliseconds
            }).ConfigureAwait(false);
    }
}
