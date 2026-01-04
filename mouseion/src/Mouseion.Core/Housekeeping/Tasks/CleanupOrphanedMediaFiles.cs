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

        // Remove media files orphaned from movies
        await connection.ExecuteAsync(@"
            DELETE FROM ""MediaFiles""
            WHERE ""Id"" IN (
                SELECT ""MediaFiles"".""Id""
                FROM ""MediaFiles""
                LEFT OUTER JOIN ""Movies"" ON ""MediaFiles"".""MovieId"" = ""Movies"".""Id""
                WHERE ""MediaFiles"".""MovieId"" > 0 AND ""Movies"".""Id"" IS NULL
            )");

        // Remove media files orphaned from books
        await connection.ExecuteAsync(@"
            DELETE FROM ""MediaFiles""
            WHERE ""Id"" IN (
                SELECT ""MediaFiles"".""Id""
                FROM ""MediaFiles""
                LEFT OUTER JOIN ""Books"" ON ""MediaFiles"".""BookId"" = ""Books"".""Id""
                WHERE ""MediaFiles"".""BookId"" > 0 AND ""Books"".""Id"" IS NULL
            )");

        // Remove media files orphaned from audiobooks
        await connection.ExecuteAsync(@"
            DELETE FROM ""MediaFiles""
            WHERE ""Id"" IN (
                SELECT ""MediaFiles"".""Id""
                FROM ""MediaFiles""
                LEFT OUTER JOIN ""Audiobooks"" ON ""MediaFiles"".""AudiobookId"" = ""Audiobooks"".""Id""
                WHERE ""MediaFiles"".""AudiobookId"" > 0 AND ""Audiobooks"".""Id"" IS NULL
            )");
    }
}
