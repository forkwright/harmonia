// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Notifications;

public class NotificationRepository : BasicRepository<NotificationDefinition>, INotificationRepository
{
    // Whitelist of valid column names for trigger queries (prevents SQL injection)
    private static readonly HashSet<string> ValidTriggerColumns = new(StringComparer.Ordinal)
    {
        "OnGrab", "OnDownload", "OnRename", "OnMediaAdded",
        "OnMediaDeleted", "OnHealthIssue", "OnHealthRestored", "OnApplicationUpdate"
    };

    public NotificationRepository(IDatabase database)
        : base(database, "Notifications")
    {
    }

    public async Task<IEnumerable<NotificationDefinition>> GetEnabledAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<NotificationDefinition>(
            "SELECT * FROM \"Notifications\" WHERE \"Enabled\" = true").ConfigureAwait(false);
    }

    public async Task<IEnumerable<NotificationDefinition>> GetByImplementationAsync(
        string implementation,
        CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<NotificationDefinition>(
            "SELECT * FROM \"Notifications\" WHERE \"Implementation\" = @Implementation",
            new { Implementation = implementation }).ConfigureAwait(false);
    }

    public async Task<IEnumerable<NotificationDefinition>> GetByTriggerAsync(
        NotificationTrigger trigger,
        CancellationToken ct = default)
    {
        var column = GetTriggerColumnName(trigger);

        // Validate column is in whitelist (defense in depth - switch already guarantees this)
        if (!ValidTriggerColumns.Contains(column))
        {
            throw new InvalidOperationException($"Invalid trigger column: {column}");
        }

        using var conn = _database.OpenConnection();

        // Column name is from a fixed whitelist, not user input - safe to interpolate
        var sql = $"SELECT * FROM \"Notifications\" WHERE \"Enabled\" = true AND \"{column}\" = true";
        return await conn.QueryAsync<NotificationDefinition>(sql).ConfigureAwait(false);
    }

    /// <summary>
    /// Maps trigger enum to database column name (used for dynamic query building)
    /// </summary>
    public static string GetTriggerColumnName(NotificationTrigger trigger)
    {
        return trigger switch
        {
            NotificationTrigger.OnGrab => "OnGrab",
            NotificationTrigger.OnDownload => "OnDownload",
            NotificationTrigger.OnRename => "OnRename",
            NotificationTrigger.OnMediaAdded => "OnMediaAdded",
            NotificationTrigger.OnMediaDeleted => "OnMediaDeleted",
            NotificationTrigger.OnHealthIssue => "OnHealthIssue",
            NotificationTrigger.OnHealthRestored => "OnHealthRestored",
            NotificationTrigger.OnApplicationUpdate => "OnApplicationUpdate",
            _ => throw new ArgumentOutOfRangeException(nameof(trigger), trigger, "Unknown notification trigger")
        };
    }
}
