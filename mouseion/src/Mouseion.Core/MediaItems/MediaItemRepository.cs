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
}
