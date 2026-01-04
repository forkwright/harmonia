// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedImportListItems : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Import List Items";

    public CleanupOrphanedImportListItems(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Remove import list items for deleted import lists
        await connection.ExecuteAsync(@"
            DELETE FROM ""ImportListItems""
            WHERE ""Id"" IN (
                SELECT ""ImportListItems"".""Id""
                FROM ""ImportListItems""
                LEFT OUTER JOIN ""ImportLists"" ON ""ImportListItems"".""ImportListId"" = ""ImportLists"".""Id""
                WHERE ""ImportLists"".""Id"" IS NULL
            )");
    }
}
