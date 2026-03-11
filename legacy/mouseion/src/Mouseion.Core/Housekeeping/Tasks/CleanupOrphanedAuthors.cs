// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedAuthors : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Authors";

    public CleanupOrphanedAuthors(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Remove authors not referenced by any MediaItem
        // MediaItems stores author info per-item; Authors table is metadata cache.
        // An author with no associated MediaItems (via author name/ID match) is orphaned.
        await connection.ExecuteAsync(@"
            DELETE FROM ""Authors""
            WHERE ""Id"" NOT IN (
                SELECT DISTINCT ""AuthorId"" FROM ""MediaItems"" WHERE ""AuthorId"" IS NOT NULL
            )");
    }
}
