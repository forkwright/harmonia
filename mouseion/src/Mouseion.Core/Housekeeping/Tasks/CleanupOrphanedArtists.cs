// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupOrphanedArtists : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Orphaned Artists";

    public CleanupOrphanedArtists(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Remove artists with no albums
        await connection.ExecuteAsync(@"
            DELETE FROM ""Artists""
            WHERE ""Id"" NOT IN (
                SELECT DISTINCT ""ArtistId"" FROM ""Albums"" WHERE ""ArtistId"" IS NOT NULL
            )");
    }
}
