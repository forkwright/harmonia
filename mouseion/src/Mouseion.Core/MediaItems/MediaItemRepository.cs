// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.Books;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Movies;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.MediaItems;

public interface IMediaItemRepository
{
    Task<MediaItem?> FindByIdAsync(int id, CancellationToken ct = default);
    Task<List<MediaItemSummary>> GetPageAsync(int page, int pageSize, MediaType? mediaType = null, CancellationToken ct = default);
    Task<int> CountAsync(MediaType? mediaType = null, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task<List<MediaItemSummary>> GetModifiedSinceAsync(DateTime modifiedSince, MediaType? mediaType = null, CancellationToken ct = default);
}

public class MediaItemSummary
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public MediaType MediaType { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string Path { get; set; } = string.Empty;
    public DateTime Added { get; set; }
    public DateTime? LastModified { get; set; }
}

public class MediaItemRepository : IMediaItemRepository
{
    private readonly IDatabase _database;

    public MediaItemRepository(IDatabase database)
    {
        _database = database;
    }

    public async Task<MediaItem?> FindByIdAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        // First, get the MediaType
        var mediaType = await conn.QueryFirstOrDefaultAsync<int>(
            "SELECT \"MediaType\" FROM \"MediaItems\" WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);

        // Query based on media type
        return mediaType switch
        {
            (int)MediaType.Book => await conn.QueryFirstOrDefaultAsync<Book>(
                "SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id",
                new { Id = id }).ConfigureAwait(false),
            (int)MediaType.Audiobook => await conn.QueryFirstOrDefaultAsync<Audiobook>(
                "SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id",
                new { Id = id }).ConfigureAwait(false),
            (int)MediaType.Movie => await conn.QueryFirstOrDefaultAsync<Movie>(
                "SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id",
                new { Id = id }).ConfigureAwait(false),
            _ => null
        };
    }

    public async Task<List<MediaItemSummary>> GetPageAsync(int page, int pageSize, MediaType? mediaType = null, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var offset = (page - 1) * pageSize;

        // Use parameterized query with conditional filtering
        const string queryWithFilter = """
            SELECT "Id", "Title", "Year", "MediaType", "Monitored", "QualityProfileId", "Path", "Added", "LastModified"
            FROM "MediaItems"
            WHERE "MediaType" = @MediaType
            ORDER BY "Added" DESC
            LIMIT @PageSize OFFSET @Offset
            """;

        const string queryNoFilter = """
            SELECT "Id", "Title", "Year", "MediaType", "Monitored", "QualityProfileId", "Path", "Added", "LastModified"
            FROM "MediaItems"
            ORDER BY "Added" DESC
            LIMIT @PageSize OFFSET @Offset
            """;

        var results = mediaType.HasValue
            ? await conn.QueryAsync<MediaItemSummary>(queryWithFilter, new
            {
                MediaType = (int)mediaType.Value,
                PageSize = pageSize,
                Offset = offset
            }).ConfigureAwait(false)
            : await conn.QueryAsync<MediaItemSummary>(queryNoFilter, new
            {
                PageSize = pageSize,
                Offset = offset
            }).ConfigureAwait(false);

        return results.ToList();
    }

    public async Task<int> CountAsync(MediaType? mediaType = null, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        const string queryWithFilter = """SELECT COUNT(*) FROM "MediaItems" WHERE "MediaType" = @MediaType""";
        const string queryNoFilter = """SELECT COUNT(*) FROM "MediaItems" """;

        return mediaType.HasValue
            ? await conn.ExecuteScalarAsync<int>(queryWithFilter, new { MediaType = (int)mediaType.Value }).ConfigureAwait(false)
            : await conn.ExecuteScalarAsync<int>(queryNoFilter).ConfigureAwait(false);
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync("DELETE FROM \"MediaItems\" WHERE \"Id\" = @Id", new { Id = id }).ConfigureAwait(false);
    }

    public async Task<List<MediaItemSummary>> GetModifiedSinceAsync(DateTime modifiedSince, MediaType? mediaType = null, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        const string queryWithFilter = """
            SELECT "Id", "Title", "Year", "MediaType", "Monitored", "QualityProfileId", "Path", "Added", "LastModified"
            FROM "MediaItems"
            WHERE "LastModified" > @ModifiedSince AND "MediaType" = @MediaType
            ORDER BY "LastModified" DESC
            """;

        const string queryNoFilter = """
            SELECT "Id", "Title", "Year", "MediaType", "Monitored", "QualityProfileId", "Path", "Added", "LastModified"
            FROM "MediaItems"
            WHERE "LastModified" > @ModifiedSince
            ORDER BY "LastModified" DESC
            """;

        var results = mediaType.HasValue
            ? await conn.QueryAsync<MediaItemSummary>(queryWithFilter, new
            {
                ModifiedSince = modifiedSince,
                MediaType = (int)mediaType.Value
            }).ConfigureAwait(false)
            : await conn.QueryAsync<MediaItemSummary>(queryNoFilter, new
            {
                ModifiedSince = modifiedSince
            }).ConfigureAwait(false);

        return results.ToList();
    }
}
