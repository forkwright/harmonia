// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authentication;

public interface IOidcProviderRepository : IBasicRepository<OidcProvider>
{
    Task<OidcProvider?> GetBySlugAsync(string slug, CancellationToken ct = default);
    Task<List<OidcProvider>> GetEnabledAsync(CancellationToken ct = default);
    Task<bool> SlugExistsAsync(string slug, CancellationToken ct = default);
}

public class OidcProviderRepository : BasicRepository<OidcProvider>, IOidcProviderRepository
{
    public OidcProviderRepository(IDatabase database)
        : base(database, "OidcProviders")
    {
    }

    public async Task<OidcProvider?> GetBySlugAsync(string slug, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<OidcProvider>(
            @"SELECT * FROM ""OidcProviders"" WHERE LOWER(""Slug"") = LOWER(@Slug)",
            new { Slug = slug }).ConfigureAwait(false);
    }

    public async Task<List<OidcProvider>> GetEnabledAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<OidcProvider>(
            @"SELECT * FROM ""OidcProviders"" WHERE ""Enabled"" = 1 ORDER BY ""SortOrder"", ""Name""")
            .ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<bool> SlugExistsAsync(string slug, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""OidcProviders"" WHERE LOWER(""Slug"") = LOWER(@Slug)",
            new { Slug = slug }).ConfigureAwait(false) > 0;
    }
}

public interface IOidcAuthStateRepository : IBasicRepository<OidcAuthState>
{
    Task<OidcAuthState?> GetByStateAsync(string state, CancellationToken ct = default);
    Task CleanupExpiredAsync(CancellationToken ct = default);
}

public class OidcAuthStateRepository : BasicRepository<OidcAuthState>, IOidcAuthStateRepository
{
    public OidcAuthStateRepository(IDatabase database)
        : base(database, "OidcAuthStates")
    {
    }

    public async Task<OidcAuthState?> GetByStateAsync(string state, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<OidcAuthState>(
            @"SELECT * FROM ""OidcAuthStates"" WHERE ""State"" = @State AND ""ExpiresAt"" > @Now",
            new { State = state, Now = DateTime.UtcNow }).ConfigureAwait(false);
    }

    public async Task CleanupExpiredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"DELETE FROM ""OidcAuthStates"" WHERE ""ExpiresAt"" < @Now",
            new { Now = DateTime.UtcNow }).ConfigureAwait(false);
    }
}
