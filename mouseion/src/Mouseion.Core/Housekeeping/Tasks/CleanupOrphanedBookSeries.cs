// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedBookSeries : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Book Series";

    public CleanupOrphanedBookSeries(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Remove book-series links for deleted books
        await connection.ExecuteAsync(@"
            DELETE FROM ""BookSeriesLinks""
            WHERE ""Id"" IN (
                SELECT ""BookSeriesLinks"".""Id""
                FROM ""BookSeriesLinks""
                LEFT OUTER JOIN ""Books"" ON ""BookSeriesLinks"".""BookId"" = ""Books"".""Id""
                WHERE ""Books"".""Id"" IS NULL
            )");

        // Remove book-series links for deleted series
        await connection.ExecuteAsync(@"
            DELETE FROM ""BookSeriesLinks""
            WHERE ""Id"" IN (
                SELECT ""BookSeriesLinks"".""Id""
                FROM ""BookSeriesLinks""
                LEFT OUTER JOIN ""BookSeries"" ON ""BookSeriesLinks"".""SeriesId"" = ""BookSeries"".""Id""
                WHERE ""BookSeries"".""Id"" IS NULL
            )");

        // Remove empty series (no books)
        await connection.ExecuteAsync(@"
            DELETE FROM ""BookSeries""
            WHERE ""Id"" IN (
                SELECT ""BookSeries"".""Id""
                FROM ""BookSeries""
                LEFT OUTER JOIN ""BookSeriesLinks"" ON ""BookSeries"".""Id"" = ""BookSeriesLinks"".""SeriesId""
                WHERE ""BookSeriesLinks"".""Id"" IS NULL
            )");
    }
}
