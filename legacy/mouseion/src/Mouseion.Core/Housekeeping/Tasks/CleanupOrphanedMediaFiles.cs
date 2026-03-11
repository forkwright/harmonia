// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedMediaFiles : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Media Files";

    public CleanupOrphanedMediaFiles(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Remove media files not associated with any MediaItem
        await connection.ExecuteAsync(@"
            DELETE FROM ""MediaFiles""
            WHERE ""MediaItemId"" IS NOT NULL
            AND ""MediaItemId"" NOT IN (SELECT ""Id"" FROM ""MediaItems"")");
    }
}
