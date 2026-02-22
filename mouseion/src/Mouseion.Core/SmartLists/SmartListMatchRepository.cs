// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.SmartLists;

public interface ISmartListMatchRepository : IBasicRepository<SmartListMatch>
{
    Task<IEnumerable<SmartListMatch>> GetByListIdAsync(int smartListId, CancellationToken ct = default);
    Task<SmartListMatch?> FindByExternalIdAsync(int smartListId, string externalId, CancellationToken ct = default);
    Task<int> CountByStatusAsync(int smartListId, SmartListMatchStatus status, CancellationToken ct = default);
    Task DeleteByListIdAsync(int smartListId, CancellationToken ct = default);
}

public class SmartListMatchRepository : BasicRepository<SmartListMatch>, ISmartListMatchRepository
{
    public SmartListMatchRepository(IDatabase database) : base(database, "SmartListMatches") { }

    public async Task<IEnumerable<SmartListMatch>> GetByListIdAsync(int smartListId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<SmartListMatch>(
            @"SELECT * FROM ""SmartListMatches"" WHERE ""SmartListId"" = @SmartListId ORDER BY ""DiscoveredAt"" DESC",
            new { SmartListId = smartListId }
        ).ConfigureAwait(false);
    }

    public async Task<SmartListMatch?> FindByExternalIdAsync(int smartListId, string externalId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<SmartListMatch>(
            @"SELECT * FROM ""SmartListMatches"" WHERE ""SmartListId"" = @SmartListId AND ""ExternalId"" = @ExternalId",
            new { SmartListId = smartListId, ExternalId = externalId }
        ).ConfigureAwait(false);
    }

    public async Task<int> CountByStatusAsync(int smartListId, SmartListMatchStatus status, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""SmartListMatches"" WHERE ""SmartListId"" = @SmartListId AND ""Status"" = @Status",
            new { SmartListId = smartListId, Status = (int)status }
        ).ConfigureAwait(false);
    }

    public async Task DeleteByListIdAsync(int smartListId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"DELETE FROM ""SmartListMatches"" WHERE ""SmartListId"" = @SmartListId",
            new { SmartListId = smartListId }
        ).ConfigureAwait(false);
    }
}
