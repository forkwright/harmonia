// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Data.Sqlite;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Api.ClientLog;

[ApiController]
[Route("api/v3/clientlog")]
[Authorize]
public class ClientLogController : ControllerBase
{
    private readonly string _dbPath;
    private static bool _initialized;
    private static readonly object _initLock = new();

    public ClientLogController(IAppFolderInfo appFolderInfo)
    {
        _dbPath = Path.Combine(appFolderInfo.AppDataFolder, "logs.db");
        EnsureTable();
    }

    private void EnsureTable()
    {
        if (_initialized) return;
        lock (_initLock)
        {
            if (_initialized) return;
            using var conn = new SqliteConnection($"Data Source={_dbPath}");
            conn.Open();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"
                CREATE TABLE IF NOT EXISTS ClientLog (
                    Id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                    Timestamp TEXT NOT NULL,
                    Level TEXT NOT NULL,
                    Source TEXT NOT NULL,
                    Message TEXT NOT NULL,
                    Detail TEXT,
                    Url TEXT,
                    Stack TEXT,
                    UserAgent TEXT,
                    ReceivedAt TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS IX_ClientLog_Timestamp ON ClientLog (Timestamp DESC);
                CREATE INDEX IF NOT EXISTS IX_ClientLog_Level ON ClientLog (Level);
            ";
            cmd.ExecuteNonQuery();
            _initialized = true;
        }
    }

    /// <summary>
    /// Receive a batch of client-side log entries.
    /// </summary>
    [HttpPost]
    public ActionResult IngestBatch([FromBody] List<ClientLogEntry> entries)
    {
        if (entries == null || entries.Count == 0)
            return Ok(new { ingested = 0 });

        // Cap batch size to prevent abuse
        if (entries.Count > 200)
            entries = entries.Take(200).ToList();

        var userAgent = Request.Headers.UserAgent.ToString();

        using var conn = new SqliteConnection($"Data Source={_dbPath}");
        conn.Open();
        using var tx = conn.BeginTransaction();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            INSERT INTO ClientLog (Timestamp, Level, Source, Message, Detail, Url, Stack, UserAgent)
            VALUES (@ts, @level, @source, @message, @detail, @url, @stack, @ua)
        ";
        cmd.Parameters.Add(new SqliteParameter("@ts", ""));
        cmd.Parameters.Add(new SqliteParameter("@level", ""));
        cmd.Parameters.Add(new SqliteParameter("@source", ""));
        cmd.Parameters.Add(new SqliteParameter("@message", ""));
        cmd.Parameters.Add(new SqliteParameter("@detail", ""));
        cmd.Parameters.Add(new SqliteParameter("@url", ""));
        cmd.Parameters.Add(new SqliteParameter("@stack", ""));
        cmd.Parameters.Add(new SqliteParameter("@ua", userAgent));

        foreach (var entry in entries)
        {
            cmd.Parameters["@ts"].Value = entry.Timestamp ?? "";
            cmd.Parameters["@level"].Value = entry.Level ?? "error";
            cmd.Parameters["@source"].Value = entry.Source ?? "unknown";
            cmd.Parameters["@message"].Value = (entry.Message ?? "").Length > 2000
                ? entry.Message![..2000] : entry.Message ?? "";
            cmd.Parameters["@detail"].Value = (object?)(entry.Detail?.Length > 4000
                ? entry.Detail[..4000] : entry.Detail) ?? DBNull.Value;
            cmd.Parameters["@url"].Value = (object?)entry.Url ?? DBNull.Value;
            cmd.Parameters["@stack"].Value = (object?)(entry.Stack?.Length > 4000
                ? entry.Stack[..4000] : entry.Stack) ?? DBNull.Value;
            cmd.ExecuteNonQuery();
        }

        tx.Commit();

        // Prune old entries — keep last 5000
        using var pruneCmd = conn.CreateCommand();
        pruneCmd.CommandText = @"
            DELETE FROM ClientLog WHERE Id NOT IN (
                SELECT Id FROM ClientLog ORDER BY Id DESC LIMIT 5000
            )
        ";
        pruneCmd.ExecuteNonQuery();

        return Ok(new { ingested = entries.Count });
    }

    /// <summary>
    /// Get recent client log entries.
    /// </summary>
    [HttpGet]
    public ActionResult<List<ClientLogRow>> GetLogs(
        [FromQuery] int limit = 100,
        [FromQuery] string? level = null,
        [FromQuery] string? source = null,
        [FromQuery] string? since = null)
    {
        if (limit > 500) limit = 500;

        using var conn = new SqliteConnection($"Data Source={_dbPath}");
        conn.Open();

        var where = new List<string>();
        var parameters = new List<SqliteParameter>();

        if (!string.IsNullOrEmpty(level))
        {
            where.Add("Level = @level");
            parameters.Add(new SqliteParameter("@level", level));
        }
        if (!string.IsNullOrEmpty(source))
        {
            where.Add("Source LIKE @source");
            parameters.Add(new SqliteParameter("@source", $"%{source}%"));
        }
        if (!string.IsNullOrEmpty(since))
        {
            where.Add("Timestamp >= @since");
            parameters.Add(new SqliteParameter("@since", since));
        }

        var whereClause = where.Count > 0 ? $"WHERE {string.Join(" AND ", where)}" : "";

        using var cmd = conn.CreateCommand();
        cmd.CommandText = $@"
            SELECT Id, Timestamp, Level, Source, Message, Detail, Url, Stack, UserAgent, ReceivedAt
            FROM ClientLog
            {whereClause}
            ORDER BY Id DESC
            LIMIT @limit
        ";
        cmd.Parameters.Add(new SqliteParameter("@limit", limit));
        foreach (var p in parameters) cmd.Parameters.Add(p);

        var results = new List<ClientLogRow>();
        using var reader = cmd.ExecuteReader();
        while (reader.Read())
        {
            results.Add(new ClientLogRow
            {
                Id = reader.GetInt64(0),
                Timestamp = reader.GetString(1),
                Level = reader.GetString(2),
                Source = reader.GetString(3),
                Message = reader.GetString(4),
                Detail = reader.IsDBNull(5) ? null : reader.GetString(5),
                Url = reader.IsDBNull(6) ? null : reader.GetString(6),
                Stack = reader.IsDBNull(7) ? null : reader.GetString(7),
                UserAgent = reader.IsDBNull(8) ? null : reader.GetString(8),
                ReceivedAt = reader.GetString(9),
            });
        }

        return Ok(results);
    }

    /// <summary>
    /// Clear all client logs.
    /// </summary>
    [HttpDelete]
    public ActionResult ClearLogs()
    {
        using var conn = new SqliteConnection($"Data Source={_dbPath}");
        conn.Open();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "DELETE FROM ClientLog";
        var deleted = cmd.ExecuteNonQuery();
        return Ok(new { deleted });
    }
}

public class ClientLogEntry
{
    public string? Timestamp { get; set; }
    public string? Level { get; set; }
    public string? Source { get; set; }
    public string? Message { get; set; }
    public string? Detail { get; set; }
    public string? Url { get; set; }
    public string? Stack { get; set; }
}

public class ClientLogRow
{
    public long Id { get; set; }
    public string Timestamp { get; set; } = "";
    public string Level { get; set; } = "";
    public string Source { get; set; } = "";
    public string Message { get; set; } = "";
    public string? Detail { get; set; }
    public string? Url { get; set; }
    public string? Stack { get; set; }
    public string? UserAgent { get; set; }
    public string ReceivedAt { get; set; } = "";
}
