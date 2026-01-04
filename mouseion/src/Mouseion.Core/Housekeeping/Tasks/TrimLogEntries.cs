// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.Datastore.Migration.Framework;

namespace Mouseion.Core.Housekeeping.Tasks;

public class TrimLogEntries : IHousekeepingTask
{
    private readonly IDbFactory _dbFactory;
    private const int MaxLogDays = 30;

    public string Name => "Trim Old Log Entries";

    public TrimLogEntries(IDbFactory dbFactory)
    {
        _dbFactory = dbFactory;
    }

    public async Task CleanAsync(CancellationToken cancellationToken = default)
    {
        var logDatabase = _dbFactory.Create(MigrationType.Log);
        using var connection = logDatabase.OpenConnection();

        var cutoffDate = DateTime.UtcNow.AddDays(-MaxLogDays);

        await connection.ExecuteAsync(
            "DELETE FROM \"Logs\" WHERE \"Time\" < @CutoffDate",
            new { CutoffDate = cutoffDate });
    }
}
