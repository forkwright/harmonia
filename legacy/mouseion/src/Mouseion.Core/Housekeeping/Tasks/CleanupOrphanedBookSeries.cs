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

        // Remove book-series links where the referenced book no longer exists as a MediaItem
        await connection.ExecuteAsync(@"
            DELETE FROM ""BookSeriesLinks""
            WHERE ""BookId"" NOT IN (SELECT ""Id"" FROM ""MediaItems"")");

        // Remove book-series links for deleted series
        await connection.ExecuteAsync(@"
            DELETE FROM ""BookSeriesLinks""
            WHERE ""SeriesId"" NOT IN (SELECT ""Id"" FROM ""BookSeries"")");

        // Remove empty series (no remaining links)
        await connection.ExecuteAsync(@"
            DELETE FROM ""BookSeries""
            WHERE ""Id"" NOT IN (
                SELECT DISTINCT ""SeriesId"" FROM ""BookSeriesLinks""
            )");
    }
}
