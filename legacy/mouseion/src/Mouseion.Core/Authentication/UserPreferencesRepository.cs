// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authentication;

public interface IUserPreferencesRepository : IBasicRepository<UserPreferences>
{
    Task<UserPreferences?> GetByUserIdAsync(int userId, CancellationToken ct = default);
    Task UpsertAsync(UserPreferences preferences, CancellationToken ct = default);
}

public class UserPreferencesRepository : BasicRepository<UserPreferences>, IUserPreferencesRepository
{
    private new readonly IDatabase _database;

    public UserPreferencesRepository(IDatabase database) : base(database, "UserPreferences")
    {
        _database = database;
    }

    public async Task<UserPreferences?> GetByUserIdAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<UserPreferences>(
            @"SELECT * FROM ""UserPreferences"" WHERE ""UserId"" = @UserId",
            new { UserId = userId }).ConfigureAwait(false);
    }

    public async Task UpsertAsync(UserPreferences preferences, CancellationToken ct = default)
    {
        var existing = await GetByUserIdAsync(preferences.UserId, ct).ConfigureAwait(false);
        if (existing == null)
        {
            preferences.CreatedAt = DateTime.UtcNow;
            preferences.UpdatedAt = DateTime.UtcNow;
            Insert(preferences);
        }
        else
        {
            preferences.Id = existing.Id;
            preferences.CreatedAt = existing.CreatedAt;
            preferences.UpdatedAt = DateTime.UtcNow;
            Update(preferences);
        }
    }
}

public interface IUserSmartListSubscriptionRepository : IBasicRepository<UserSmartListSubscription>
{
    Task<List<UserSmartListSubscription>> GetByUserIdAsync(int userId, CancellationToken ct = default);
    Task<List<UserSmartListSubscription>> GetBySmartListIdAsync(int smartListId, CancellationToken ct = default);
    Task<UserSmartListSubscription?> GetSubscriptionAsync(int userId, int smartListId, CancellationToken ct = default);
    Task DeleteSubscriptionAsync(int userId, int smartListId, CancellationToken ct = default);
}

public class UserSmartListSubscriptionRepository : BasicRepository<UserSmartListSubscription>, IUserSmartListSubscriptionRepository
{
    private new readonly IDatabase _database;

    public UserSmartListSubscriptionRepository(IDatabase database) : base(database, "UserSmartListSubscriptions")
    {
        _database = database;
    }

    public async Task<List<UserSmartListSubscription>> GetByUserIdAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<UserSmartListSubscription>(
            @"SELECT * FROM ""UserSmartListSubscriptions"" WHERE ""UserId"" = @UserId",
            new { UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<UserSmartListSubscription>> GetBySmartListIdAsync(int smartListId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<UserSmartListSubscription>(
            @"SELECT * FROM ""UserSmartListSubscriptions"" WHERE ""SmartListId"" = @SmartListId",
            new { SmartListId = smartListId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<UserSmartListSubscription?> GetSubscriptionAsync(int userId, int smartListId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<UserSmartListSubscription>(
            @"SELECT * FROM ""UserSmartListSubscriptions"" WHERE ""UserId"" = @UserId AND ""SmartListId"" = @SmartListId",
            new { UserId = userId, SmartListId = smartListId }).ConfigureAwait(false);
    }

    public async Task DeleteSubscriptionAsync(int userId, int smartListId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"DELETE FROM ""UserSmartListSubscriptions"" WHERE ""UserId"" = @UserId AND ""SmartListId"" = @SmartListId",
            new { UserId = userId, SmartListId = smartListId }).ConfigureAwait(false);
    }
}

public interface IUserPermissionRepository : IBasicRepository<UserPermission>
{
    Task<List<UserPermission>> GetByUserIdAsync(int userId, CancellationToken ct = default);
    Task<List<UserPermission>> GetByTypeAsync(int userId, PermissionType type, CancellationToken ct = default);
    Task DeleteAllForUserAsync(int userId, CancellationToken ct = default);
}

public class UserPermissionRepository : BasicRepository<UserPermission>, IUserPermissionRepository
{
    private new readonly IDatabase _database;

    public UserPermissionRepository(IDatabase database) : base(database, "UserPermissions")
    {
        _database = database;
    }

    public async Task<List<UserPermission>> GetByUserIdAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<UserPermission>(
            @"SELECT * FROM ""UserPermissions"" WHERE ""UserId"" = @UserId",
            new { UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<UserPermission>> GetByTypeAsync(int userId, PermissionType type, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<UserPermission>(
            @"SELECT * FROM ""UserPermissions"" WHERE ""UserId"" = @UserId AND ""PermissionType"" = @Type",
            new { UserId = userId, Type = (int)type }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task DeleteAllForUserAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"DELETE FROM ""UserPermissions"" WHERE ""UserId"" = @UserId",
            new { UserId = userId }).ConfigureAwait(false);
    }
}

public interface IApiKeyRepository : IBasicRepository<ApiKey>
{
    Task<List<ApiKey>> GetByUserIdAsync(int userId, CancellationToken ct = default);
    Task<ApiKey?> GetByPrefixAsync(string prefix, CancellationToken ct = default);
    Task<List<ApiKey>> GetActiveAsync(CancellationToken ct = default);
    Task UpdateLastUsedAsync(int id, CancellationToken ct = default);
}

public class ApiKeyRepository : BasicRepository<ApiKey>, IApiKeyRepository
{
    private new readonly IDatabase _database;

    public ApiKeyRepository(IDatabase database) : base(database, "ApiKeys")
    {
        _database = database;
    }

    public async Task<List<ApiKey>> GetByUserIdAsync(int userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ApiKey>(
            @"SELECT * FROM ""ApiKeys"" WHERE ""UserId"" = @UserId ORDER BY ""CreatedAt"" DESC",
            new { UserId = userId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<ApiKey?> GetByPrefixAsync(string prefix, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<ApiKey>(
            @"SELECT * FROM ""ApiKeys"" WHERE ""KeyPrefix"" = @Prefix AND ""IsActive"" = 1",
            new { Prefix = prefix }).ConfigureAwait(false);
    }

    public async Task<List<ApiKey>> GetActiveAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ApiKey>(
            @"SELECT * FROM ""ApiKeys"" WHERE ""IsActive"" = 1 ORDER BY ""LastUsedAt"" DESC")
            .ConfigureAwait(false);
        return result.ToList();
    }

    public async Task UpdateLastUsedAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"UPDATE ""ApiKeys"" SET ""LastUsedAt"" = @Now WHERE ""Id"" = @Id",
            new { Id = id, Now = DateTime.UtcNow }).ConfigureAwait(false);
    }
}

public interface IAuditLogRepository
{
    Task InsertAsync(AuditLogEntry entry, CancellationToken ct = default);
    Task<List<AuditLogEntry>> GetRecentAsync(int count = 100, CancellationToken ct = default);
    Task<List<AuditLogEntry>> GetByUserIdAsync(int userId, int count = 50, CancellationToken ct = default);
    Task<List<AuditLogEntry>> GetByActionAsync(string action, int count = 50, CancellationToken ct = default);
    Task PurgeOlderThanAsync(DateTime cutoff, CancellationToken ct = default);
}

public class AuditLogRepository : IAuditLogRepository
{
    private readonly IDatabase _database;

    public AuditLogRepository(IDatabase database)
    {
        _database = database;
    }

    public async Task InsertAsync(AuditLogEntry entry, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"INSERT INTO ""AuditLog"" (""UserId"", ""Action"", ""ResourceType"", ""ResourceId"", ""Details"", ""IpAddress"", ""UserAgent"", ""Timestamp"")
              VALUES (@UserId, @Action, @ResourceType, @ResourceId, @Details, @IpAddress, @UserAgent, @Timestamp)",
            entry).ConfigureAwait(false);
    }

    public async Task<List<AuditLogEntry>> GetRecentAsync(int count = 100, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AuditLogEntry>(
            @"SELECT * FROM ""AuditLog"" ORDER BY ""Timestamp"" DESC LIMIT @Count",
            new { Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AuditLogEntry>> GetByUserIdAsync(int userId, int count = 50, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AuditLogEntry>(
            @"SELECT * FROM ""AuditLog"" WHERE ""UserId"" = @UserId ORDER BY ""Timestamp"" DESC LIMIT @Count",
            new { UserId = userId, Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AuditLogEntry>> GetByActionAsync(string action, int count = 50, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AuditLogEntry>(
            @"SELECT * FROM ""AuditLog"" WHERE ""Action"" = @Action ORDER BY ""Timestamp"" DESC LIMIT @Count",
            new { Action = action, Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task PurgeOlderThanAsync(DateTime cutoff, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"DELETE FROM ""AuditLog"" WHERE ""Timestamp"" < @Cutoff",
            new { Cutoff = cutoff }).ConfigureAwait(false);
    }
}
