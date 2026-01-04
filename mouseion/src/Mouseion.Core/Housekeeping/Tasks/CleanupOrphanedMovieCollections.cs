// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedMovieCollections : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Movie Collections";

    public CleanupOrphanedMovieCollections(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Remove movie-collection links for deleted movies
        await connection.ExecuteAsync(@"
            DELETE FROM ""MovieCollections""
            WHERE ""Id"" IN (
                SELECT ""MovieCollections"".""Id""
                FROM ""MovieCollections""
                LEFT OUTER JOIN ""Movies"" ON ""MovieCollections"".""MovieId"" = ""Movies"".""Id""
                WHERE ""Movies"".""Id"" IS NULL
            )");

        // Remove movie-collection links for deleted collections
        await connection.ExecuteAsync(@"
            DELETE FROM ""MovieCollections""
            WHERE ""Id"" IN (
                SELECT ""MovieCollections"".""Id""
                FROM ""MovieCollections""
                LEFT OUTER JOIN ""Collections"" ON ""MovieCollections"".""CollectionId"" = ""Collections"".""Id""
                WHERE ""Collections"".""Id"" IS NULL
            )");

        // Remove empty collections (no movies)
        await connection.ExecuteAsync(@"
            DELETE FROM ""Collections""
            WHERE ""Id"" IN (
                SELECT ""Collections"".""Id""
                FROM ""Collections""
                LEFT OUTER JOIN ""MovieCollections"" ON ""Collections"".""Id"" = ""MovieCollections"".""CollectionId""
                WHERE ""MovieCollections"".""Id"" IS NULL
            )");
    }
}
