// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Progress;

public interface IPlaybackQueueRepository : IBasicRepository<PlaybackQueue>
{
    Task<PlaybackQueue?> GetByUserAndDeviceAsync(int userId, string deviceName, CancellationToken ct = default);
    Task<List<PlaybackQueue>> GetByUserAsync(int userId, CancellationToken ct = default);
    Task UpsertAsync(PlaybackQueue queue, CancellationToken ct = default);
}

public class PlaybackQueueRepository : BasicRepository<PlaybackQueue>, IPlaybackQueueRepository
{
    public PlaybackQueueRepository(IDatabase database)
        : base(database, "PlaybackQueues")
    {
    }

    public async Task<PlaybackQueue?> GetByUserAndDeviceAsync(int userId, string deviceName, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<PlaybackQueue>(
            @"SELECT * FROM ""PlaybackQueues""
              WHERE ""UserId"" = @UserId AND ""DeviceName"" = @DeviceName",
            new { UserId = userId, DeviceName = deviceName }).ConfigureAwait(false);
    }

    public async Task<List<PlaybackQueue>> GetByUserAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PlaybackQueue>(
            @"SELECT * FROM ""PlaybackQueues""
              WHERE ""UserId"" = @UserId
              ORDER BY ""UpdatedAt"" DESC",
            new { UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task UpsertAsync(PlaybackQueue queue, CancellationToken ct = default)
    {
        var existing = await GetByUserAndDeviceAsync(queue.UserId, queue.DeviceName, ct).ConfigureAwait(false);

        if (existing == null)
        {
            queue.UpdatedAt = DateTime.UtcNow;
            await InsertAsync(queue, ct).ConfigureAwait(false);
        }
        else
        {
            queue.Id = existing.Id;
            queue.UpdatedAt = DateTime.UtcNow;
            await UpdateAsync(queue, ct).ConfigureAwait(false);
        }
    }
}
