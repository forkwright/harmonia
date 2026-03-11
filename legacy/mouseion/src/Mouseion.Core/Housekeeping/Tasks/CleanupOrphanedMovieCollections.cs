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

        // Remove collection links where the movie MediaItem no longer exists
        await connection.ExecuteAsync(@"
            DELETE FROM ""MovieCollections""
            WHERE ""MovieId"" NOT IN (SELECT ""Id"" FROM ""MediaItems"")");

        // Remove collection links for deleted collections
        await connection.ExecuteAsync(@"
            DELETE FROM ""MovieCollections""
            WHERE ""CollectionId"" NOT IN (SELECT ""Id"" FROM ""Collections"")");

        // Remove empty collections
        await connection.ExecuteAsync(@"
            DELETE FROM ""Collections""
            WHERE ""Id"" NOT IN (
                SELECT DISTINCT ""CollectionId"" FROM ""MovieCollections""
            )");
    }
}
