// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.SmartLists;

public interface ISmartListRepository : IBasicRepository<SmartList>
{
    Task<IEnumerable<SmartList>> GetEnabledAsync(CancellationToken ct = default);
    Task<IEnumerable<SmartList>> GetDueForRefreshAsync(CancellationToken ct = default);
}

public class SmartListRepository : BasicRepository<SmartList>, ISmartListRepository
{
    public SmartListRepository(IDatabase database) : base(database) { }

    public async Task<IEnumerable<SmartList>> GetEnabledAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<SmartList>(
            @"SELECT * FROM ""SmartLists"" WHERE ""Enabled"" = 1"
        ).ConfigureAwait(false);
    }

    public async Task<IEnumerable<SmartList>> GetDueForRefreshAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var now = DateTime.UtcNow;

        return await conn.QueryAsync<SmartList>(
            @"SELECT * FROM ""SmartLists"" WHERE ""Enabled"" = 1
              AND (""LastRefreshed"" IS NULL
                   OR (""RefreshInterval"" = 1 AND ""LastRefreshed"" < @Daily)
                   OR (""RefreshInterval"" = 2 AND ""LastRefreshed"" < @Weekly)
                   OR (""RefreshInterval"" = 3 AND ""LastRefreshed"" < @Biweekly)
                   OR (""RefreshInterval"" = 4 AND ""LastRefreshed"" < @Monthly))",
            new
            {
                Daily = now.AddDays(-1),
                Weekly = now.AddDays(-7),
                Biweekly = now.AddDays(-14),
                Monthly = now.AddDays(-30)
            }
        ).ConfigureAwait(false);
    }
}
