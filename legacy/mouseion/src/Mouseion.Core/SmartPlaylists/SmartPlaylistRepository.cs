// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.SmartPlaylists;

public class SmartPlaylistRepository : BasicRepository<SmartPlaylist>, ISmartPlaylistRepository
{
    public SmartPlaylistRepository(IDatabase database)
        : base(database, "SmartPlaylists")
    {
    }

    public async Task<IEnumerable<SmartPlaylistTrack>> GetTracksAsync(
        int smartPlaylistId,
        CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<SmartPlaylistTrack>(
            "SELECT * FROM \"SmartPlaylistTracks\" WHERE \"SmartPlaylistId\" = @SmartPlaylistId ORDER BY \"Position\"",
            new { SmartPlaylistId = smartPlaylistId }).ConfigureAwait(false);
    }

    public async Task SetTracksAsync(
        int smartPlaylistId,
        IList<SmartPlaylistTrack> tracks,
        CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        using var transaction = conn.BeginTransaction();

        try
        {
            await conn.ExecuteAsync(
                "DELETE FROM \"SmartPlaylistTracks\" WHERE \"SmartPlaylistId\" = @SmartPlaylistId",
                new { SmartPlaylistId = smartPlaylistId },
                transaction).ConfigureAwait(false);

            foreach (var track in tracks)
            {
                track.SmartPlaylistId = smartPlaylistId;
                await conn.ExecuteAsync(
                    "INSERT INTO \"SmartPlaylistTracks\" (\"SmartPlaylistId\", \"TrackId\", \"Position\") VALUES (@SmartPlaylistId, @TrackId, @Position)",
                    track,
                    transaction).ConfigureAwait(false);
            }

            transaction.Commit();
        }
        catch
        {
            transaction.Rollback();
            throw;
        }
    }

    public async Task<IEnumerable<SmartPlaylist>> GetStaleAsync(
        DateTime threshold,
        CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<SmartPlaylist>(
            "SELECT * FROM \"SmartPlaylists\" WHERE \"LastRefreshed\" < @Threshold",
            new { Threshold = threshold }).ConfigureAwait(false);
    }
}
