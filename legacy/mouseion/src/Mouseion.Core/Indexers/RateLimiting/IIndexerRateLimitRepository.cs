// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Indexers.RateLimiting;

public interface IIndexerRateLimitRepository
{
    IndexerRateLimit? GetByName(string indexerName);
    IndexerRateLimit Upsert(IndexerRateLimit rateLimit);
    List<IndexerRateLimit> GetAll();
    void Delete(int id);
}

public interface IIndexerRequestLogRepository
{
    void Log(IndexerRequestLog entry);
    int CountSince(string indexerName, DateTime since);
    List<IndexerRequestLog> GetRecent(string indexerName, int limit = 50);
    void PurgeBefore(DateTime before);
}
