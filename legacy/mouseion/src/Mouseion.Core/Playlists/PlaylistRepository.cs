// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Playlists;

public class Playlist
{
    public int Id { get; set; }
    public int UserId { get; set; }
    public string Name { get; set; } = null!;
    public string? Description { get; set; }
    public DateTime Created { get; set; }
    public DateTime Modified { get; set; }
}

public class PlaylistTrack
{
    public int Id { get; set; }
    public int PlaylistId { get; set; }
    public int MediaItemId { get; set; }
    public int Position { get; set; }
    public DateTime Added { get; set; }
}

public interface IPlaylistRepository
{
    Task<List<Playlist>> GetByUserAsync(int userId, int page, int pageSize, CancellationToken ct = default);
    Task<int> CountByUserAsync(int userId, CancellationToken ct = default);
    Task<Playlist?> FindAsync(int id, CancellationToken ct = default);
    Task<Playlist> CreateAsync(Playlist playlist, CancellationToken ct = default);
    Task<Playlist> UpdateAsync(Playlist playlist, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);

    Task<List<PlaylistTrack>> GetTracksAsync(int playlistId, CancellationToken ct = default);
    Task AddTrackAsync(int playlistId, int mediaItemId, CancellationToken ct = default);
    Task RemoveTrackAsync(int playlistId, int mediaItemId, CancellationToken ct = default);
    Task ReorderTracksAsync(int playlistId, List<int> mediaItemIds, CancellationToken ct = default);
}

public class PlaylistRepository : IPlaylistRepository
{
    private readonly IDatabase _database;

    public PlaylistRepository(IDatabase database)
    {
        _database = database;
    }

    public async Task<List<Playlist>> GetByUserAsync(int userId, int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        var result = await conn.QueryAsync<Playlist>(
            "SELECT * FROM \"Playlists\" WHERE \"UserId\" = @UserId ORDER BY \"Modified\" DESC LIMIT @PageSize OFFSET @Offset",
            new { UserId = userId, PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> CountByUserAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM \"Playlists\" WHERE \"UserId\" = @UserId",
            new { UserId = userId }).ConfigureAwait(false);
    }

    public async Task<Playlist?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Playlist>(
            "SELECT * FROM \"Playlists\" WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task<Playlist> CreateAsync(Playlist playlist, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var now = DateTime.UtcNow;
        playlist.Created = now;
        playlist.Modified = now;
        var id = await conn.QuerySingleAsync<int>(
            @"INSERT INTO ""Playlists"" (""UserId"", ""Name"", ""Description"", ""Created"", ""Modified"")
              VALUES (@UserId, @Name, @Description, @Created, @Modified);
              SELECT last_insert_rowid();",
            playlist).ConfigureAwait(false);
        playlist.Id = id;
        return playlist;
    }

    public async Task<Playlist> UpdateAsync(Playlist playlist, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        playlist.Modified = DateTime.UtcNow;
        await conn.ExecuteAsync(
            @"UPDATE ""Playlists"" SET ""Name"" = @Name, ""Description"" = @Description, ""Modified"" = @Modified WHERE ""Id"" = @Id",
            playlist).ConfigureAwait(false);
        return playlist;
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync("DELETE FROM \"PlaylistTracks\" WHERE \"PlaylistId\" = @Id", new { Id = id }).ConfigureAwait(false);
        await conn.ExecuteAsync("DELETE FROM \"Playlists\" WHERE \"Id\" = @Id", new { Id = id }).ConfigureAwait(false);
    }

    public async Task<List<PlaylistTrack>> GetTracksAsync(int playlistId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PlaylistTrack>(
            "SELECT * FROM \"PlaylistTracks\" WHERE \"PlaylistId\" = @PlaylistId ORDER BY \"Position\" ASC",
            new { PlaylistId = playlistId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task AddTrackAsync(int playlistId, int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var maxPos = await conn.QuerySingleOrDefaultAsync<int?>(
            "SELECT MAX(\"Position\") FROM \"PlaylistTracks\" WHERE \"PlaylistId\" = @PlaylistId",
            new { PlaylistId = playlistId }).ConfigureAwait(false);

        await conn.ExecuteAsync(
            @"INSERT INTO ""PlaylistTracks"" (""PlaylistId"", ""MediaItemId"", ""Position"", ""Added"")
              VALUES (@PlaylistId, @MediaItemId, @Position, @Added)",
            new { PlaylistId = playlistId, MediaItemId = mediaItemId, Position = (maxPos ?? 0) + 1, Added = DateTime.UtcNow }).ConfigureAwait(false);

        await conn.ExecuteAsync(
            "UPDATE \"Playlists\" SET \"Modified\" = @Modified WHERE \"Id\" = @Id",
            new { Modified = DateTime.UtcNow, Id = playlistId }).ConfigureAwait(false);
    }

    public async Task RemoveTrackAsync(int playlistId, int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "DELETE FROM \"PlaylistTracks\" WHERE \"PlaylistId\" = @PlaylistId AND \"MediaItemId\" = @MediaItemId",
            new { PlaylistId = playlistId, MediaItemId = mediaItemId }).ConfigureAwait(false);

        await conn.ExecuteAsync(
            "UPDATE \"Playlists\" SET \"Modified\" = @Modified WHERE \"Id\" = @Id",
            new { Modified = DateTime.UtcNow, Id = playlistId }).ConfigureAwait(false);
    }

    public async Task ReorderTracksAsync(int playlistId, List<int> mediaItemIds, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        using var tx = conn.BeginTransaction();

        for (var i = 0; i < mediaItemIds.Count; i++)
        {
            await conn.ExecuteAsync(
                "UPDATE \"PlaylistTracks\" SET \"Position\" = @Position WHERE \"PlaylistId\" = @PlaylistId AND \"MediaItemId\" = @MediaItemId",
                new { Position = i + 1, PlaylistId = playlistId, MediaItemId = mediaItemIds[i] },
                tx).ConfigureAwait(false);
        }

        tx.Commit();
    }
}
