// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Favorites;

public class Favorite
{
    public int Id { get; set; }
    public int UserId { get; set; }
    public int MediaItemId { get; set; }
    public DateTime Added { get; set; }
}

public interface IFavoriteRepository
{
    Task<List<int>> GetFavoriteIdsAsync(int userId, CancellationToken ct = default);
    Task<List<Favorite>> GetFavoritesPagedAsync(int userId, int page, int pageSize, CancellationToken ct = default);
    Task<int> CountAsync(int userId, CancellationToken ct = default);
    Task<bool> IsFavoriteAsync(int userId, int mediaItemId, CancellationToken ct = default);
    Task AddAsync(int userId, int mediaItemId, CancellationToken ct = default);
    Task RemoveAsync(int userId, int mediaItemId, CancellationToken ct = default);
}

public class FavoriteRepository : IFavoriteRepository
{
    private readonly IDatabase _database;

    public FavoriteRepository(IDatabase database)
    {
        _database = database;
    }

    public async Task<List<int>> GetFavoriteIdsAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<int>(
            "SELECT \"MediaItemId\" FROM \"Favorites\" WHERE \"UserId\" = @UserId ORDER BY \"Added\" DESC",
            new { UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<Favorite>> GetFavoritesPagedAsync(int userId, int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        var result = await conn.QueryAsync<Favorite>(
            "SELECT * FROM \"Favorites\" WHERE \"UserId\" = @UserId ORDER BY \"Added\" DESC LIMIT @PageSize OFFSET @Offset",
            new { UserId = userId, PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> CountAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM \"Favorites\" WHERE \"UserId\" = @UserId",
            new { UserId = userId }).ConfigureAwait(false);
    }

    public async Task<bool> IsFavoriteAsync(int userId, int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM \"Favorites\" WHERE \"UserId\" = @UserId AND \"MediaItemId\" = @MediaItemId",
            new { UserId = userId, MediaItemId = mediaItemId }).ConfigureAwait(false);
        return count > 0;
    }

    public async Task AddAsync(int userId, int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "INSERT OR IGNORE INTO \"Favorites\" (\"UserId\", \"MediaItemId\", \"Added\") VALUES (@UserId, @MediaItemId, @Added)",
            new { UserId = userId, MediaItemId = mediaItemId, Added = DateTime.UtcNow }).ConfigureAwait(false);
    }

    public async Task RemoveAsync(int userId, int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "DELETE FROM \"Favorites\" WHERE \"UserId\" = @UserId AND \"MediaItemId\" = @MediaItemId",
            new { UserId = userId, MediaItemId = mediaItemId }).ConfigureAwait(false);
    }
}
