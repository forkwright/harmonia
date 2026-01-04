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
            WHERE ""Id"" IN (
                SELECT ""Artists"".""Id""
                FROM ""Artists""
                LEFT OUTER JOIN ""Albums"" ON ""Artists"".""Id"" = ""Albums"".""ArtistId""
                WHERE ""Albums"".""Id"" IS NULL
            )");

        // Remove albums with no tracks
        await connection.ExecuteAsync(@"
            DELETE FROM ""Albums""
            WHERE ""Id"" IN (
                SELECT ""Albums"".""Id""
                FROM ""Albums""
                LEFT OUTER JOIN ""Tracks"" ON ""Albums"".""Id"" = ""Tracks"".""AlbumId""
                WHERE ""Tracks"".""Id"" IS NULL
            )");
    }
}
