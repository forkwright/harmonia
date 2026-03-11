// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Indexers.RateLimiting;

public class IndexerRateLimitRepository : BasicRepository<IndexerRateLimit>, IIndexerRateLimitRepository
{
    public IndexerRateLimitRepository(IDatabase database) : base(database, "IndexerRateLimits") { }

    public IndexerRateLimit? GetByName(string indexerName)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<IndexerRateLimit>(
            $"SELECT * FROM \"{_table}\" WHERE \"IndexerName\" = @IndexerName",
            new { IndexerName = indexerName });
    }

    public IndexerRateLimit Upsert(IndexerRateLimit rateLimit)
    {
        var existing = GetByName(rateLimit.IndexerName);
        rateLimit.UpdatedAt = DateTime.UtcNow;

        if (existing != null)
        {
            rateLimit.Id = existing.Id;
            rateLimit.CreatedAt = existing.CreatedAt;
            Update(rateLimit);
            return rateLimit;
        }

        rateLimit.CreatedAt = DateTime.UtcNow;
        return Insert(rateLimit);
    }

    public List<IndexerRateLimit> GetAll()
    {
        return All().ToList();
    }
}

public class IndexerRequestLogRepository : BasicRepository<IndexerRequestLog>, IIndexerRequestLogRepository
{
    public IndexerRequestLogRepository(IDatabase database) : base(database, "IndexerRequestLog") { }

    public void Log(IndexerRequestLog entry)
    {
        entry.RequestedAt = DateTime.UtcNow;
        Insert(entry);
    }

    public int CountSince(string indexerName, DateTime since)
    {
        using var conn = _database.OpenConnection();
        return conn.ExecuteScalar<int>(
            $"SELECT COUNT(*) FROM \"{_table}\" WHERE \"IndexerName\" = @IndexerName AND \"RequestedAt\" >= @Since",
            new { IndexerName = indexerName, Since = since });
    }

    public List<IndexerRequestLog> GetRecent(string indexerName, int limit = 50)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<IndexerRequestLog>(
            $"SELECT * FROM \"{_table}\" WHERE \"IndexerName\" = @IndexerName ORDER BY \"RequestedAt\" DESC LIMIT @Limit",
            new { IndexerName = indexerName, Limit = limit }).ToList();
    }

    public void PurgeBefore(DateTime before)
    {
        using var conn = _database.OpenConnection();
        conn.Execute(
            $"DELETE FROM \"{_table}\" WHERE \"RequestedAt\" < @Before",
            new { Before = before });
    }
}
