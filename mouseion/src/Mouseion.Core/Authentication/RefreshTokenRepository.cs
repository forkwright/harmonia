// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authentication;

public interface IRefreshTokenRepository : IBasicRepository<RefreshToken>
{
    Task<RefreshToken?> GetByTokenAsync(string token, CancellationToken ct = default);
    Task<List<RefreshToken>> GetActiveByUserIdAsync(int userId, CancellationToken ct = default);
    Task RevokeTokenAsync(string token, CancellationToken ct = default);
    Task RevokeAllForUserAsync(int userId, CancellationToken ct = default);
    Task CleanupExpiredAsync(CancellationToken ct = default);
}

public class RefreshTokenRepository : BasicRepository<RefreshToken>, IRefreshTokenRepository
{
    public RefreshTokenRepository(IDatabase database)
        : base(database, "RefreshTokens")
    {
    }

    public async Task<RefreshToken?> GetByTokenAsync(string token, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<RefreshToken>(
            @"SELECT * FROM ""RefreshTokens"" WHERE ""Token"" = @Token",
            new { Token = token }).ConfigureAwait(false);
    }

    public async Task<List<RefreshToken>> GetActiveByUserIdAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<RefreshToken>(
            @"SELECT * FROM ""RefreshTokens""
              WHERE ""UserId"" = @UserId AND ""RevokedAt"" IS NULL AND ""ExpiresAt"" > @Now
              ORDER BY ""CreatedAt"" DESC",
            new { UserId = userId, Now = DateTime.UtcNow }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task RevokeTokenAsync(string token, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"UPDATE ""RefreshTokens"" SET ""RevokedAt"" = @Now WHERE ""Token"" = @Token",
            new { Token = token, Now = DateTime.UtcNow }).ConfigureAwait(false);
    }

    public async Task RevokeAllForUserAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"UPDATE ""RefreshTokens"" SET ""RevokedAt"" = @Now
              WHERE ""UserId"" = @UserId AND ""RevokedAt"" IS NULL",
            new { UserId = userId, Now = DateTime.UtcNow }).ConfigureAwait(false);
    }

    public async Task CleanupExpiredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"DELETE FROM ""RefreshTokens"" WHERE ""ExpiresAt"" < @Cutoff",
            new { Cutoff = DateTime.UtcNow.AddDays(-7) }).ConfigureAwait(false);
    }
}
