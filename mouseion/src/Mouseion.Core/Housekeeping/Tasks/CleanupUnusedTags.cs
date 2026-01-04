// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Data;
using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Housekeeping.Tasks;

public class CleanupUnusedTags : IHousekeepingTask
{
    private readonly IDatabase _database;

    public string Name => "Cleanup Unused Tags";

    public CleanupUnusedTags(IDatabase database)
    {
        _database = database;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        using var connection = _database.OpenConnection();

        // Tables that can have tags
        var taggedTables = new[]
        {
            "Movies",
            "Books",
            "Audiobooks",
            "Artists",
            "Notifications",
            "ImportLists",
            "Indexers",
            "DownloadClients"
        };

        var usedTags = new List<int>();

        foreach (var table in taggedTables)
        {
            var tags = await GetUsedTagsFromTableAsync(connection, table);
            usedTags.AddRange(tags);
        }

        var distinctUsedTags = usedTags.Distinct().ToArray();

        if (distinctUsedTags.Any())
        {
            if (_database.DatabaseType == DatabaseType.PostgreSQL)
            {
                await connection.ExecuteAsync(
                    "DELETE FROM \"Tags\" WHERE NOT \"Id\" = ANY (@UsedTags)",
                    new { UsedTags = distinctUsedTags });
            }
            else
            {
                await connection.ExecuteAsync(
                    "DELETE FROM \"Tags\" WHERE \"Id\" NOT IN @UsedTags",
                    new { UsedTags = distinctUsedTags });
            }
        }
        else
        {
            await connection.ExecuteAsync("DELETE FROM \"Tags\"");
        }
    }

    private static async Task<IEnumerable<int>> GetUsedTagsFromTableAsync(
        IDbConnection connection,
        string table)
    {
        var query = $"SELECT DISTINCT \"Tags\" FROM \"{table}\" WHERE NOT \"Tags\" = '[]' AND NOT \"Tags\" IS NULL";
        var tagLists = await connection.QueryAsync<List<int>>(query);
        return tagLists.SelectMany(x => x).Distinct();
    }
}
