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

        // Remove authors with no books or audiobooks
        await connection.ExecuteAsync(@"
            DELETE FROM ""Authors""
            WHERE ""Id"" IN (
                SELECT ""Authors"".""Id""
                FROM ""Authors""
                LEFT OUTER JOIN ""Books"" ON ""Authors"".""Id"" = ""Books"".""AuthorId""
                LEFT OUTER JOIN ""Audiobooks"" ON ""Authors"".""Id"" = ""Audiobooks"".""AuthorId""
                WHERE ""Books"".""Id"" IS NULL AND ""Audiobooks"".""Id"" IS NULL
            )");
    }
}
