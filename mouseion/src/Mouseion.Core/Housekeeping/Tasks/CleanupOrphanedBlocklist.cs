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

        // Remove blocklist entries for deleted movies
        await connection.ExecuteAsync(@"
            DELETE FROM ""Blocklist""
            WHERE ""Id"" IN (
                SELECT ""Blocklist"".""Id""
                FROM ""Blocklist""
                LEFT OUTER JOIN ""Movies"" ON ""Blocklist"".""MovieId"" = ""Movies"".""Id""
                WHERE ""Blocklist"".""MovieId"" > 0 AND ""Movies"".""Id"" IS NULL
            )");

        // Remove blocklist entries for deleted books
        await connection.ExecuteAsync(@"
            DELETE FROM ""Blocklist""
            WHERE ""Id"" IN (
                SELECT ""Blocklist"".""Id""
                FROM ""Blocklist""
                LEFT OUTER JOIN ""Books"" ON ""Blocklist"".""BookId"" = ""Books"".""Id""
                WHERE ""Blocklist"".""BookId"" > 0 AND ""Books"".""Id"" IS NULL
            )");

        // Remove blocklist entries for deleted audiobooks
        await connection.ExecuteAsync(@"
            DELETE FROM ""Blocklist""
            WHERE ""Id"" IN (
                SELECT ""Blocklist"".""Id""
                FROM ""Blocklist""
                LEFT OUTER JOIN ""Audiobooks"" ON ""Blocklist"".""AudiobookId"" = ""Audiobooks"".""Id""
                WHERE ""Blocklist"".""AudiobookId"" > 0 AND ""Audiobooks"".""Id"" IS NULL
            )");

        // Remove blocklist entries for deleted artists/albums
        await connection.ExecuteAsync(@"
            DELETE FROM ""Blocklist""
            WHERE ""Id"" IN (
                SELECT ""Blocklist"".""Id""
                FROM ""Blocklist""
                LEFT OUTER JOIN ""Artists"" ON ""Blocklist"".""ArtistId"" = ""Artists"".""Id""
                WHERE ""Blocklist"".""ArtistId"" > 0 AND ""Artists"".""Id"" IS NULL
            )");
    }
}
