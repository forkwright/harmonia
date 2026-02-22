// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedBlocklist : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Blocklist Entries";

    public CleanupOrphanedBlocklist(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Blocklists references MediaItems via a MediaItemId column.
        // Remove entries where the referenced MediaItem no longer exists.
        try
        {
            await connection.ExecuteAsync(@"
                DELETE FROM ""Blocklists""
                WHERE ""MediaItemId"" IS NOT NULL
                AND ""MediaItemId"" NOT IN (SELECT ""Id"" FROM ""MediaItems"")");
        }
        catch
        {
            // Blocklists table may not have MediaItemId column yet — skip silently
        }
    }
}
