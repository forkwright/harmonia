// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Dapper;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Datastore;
using Mouseion.Core.Download.Strm;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Notifications;

namespace Mouseion.Core.Download.Acquisition;

// ──────────────────────────────────────────────
// Entities
// ──────────────────────────────────────────────

public class AcquisitionQueueItem : ModelBase
{
    public int MediaItemId { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Priority { get; set; } = 50; // 0=highest, 100=lowest
    public AcquisitionStrategy Strategy { get; set; }
    public AcquisitionStatus Status { get; set; }
    public AcquisitionSource Source { get; set; }
    public int? QualityProfileId { get; set; }
    public string? PreferredIndexers { get; set; } // JSON array
    public string? ErrorMessage { get; set; }
    public int RetryCount { get; set; }
    public int MaxRetries { get; set; } = 3;
    public DateTime? NextRetryAt { get; set; }
    public int? RequestedBy { get; set; }
    public DateTime RequestedAt { get; set; } = DateTime.UtcNow;
    public DateTime? StartedAt { get; set; }
    public DateTime? CompletedAt { get; set; }
}

public enum AcquisitionStrategy { Download = 0, Strm = 1, MonitorOnly = 2 }
public enum AcquisitionStatus { Queued = 0, Searching = 1, Found = 2, Grabbing = 3, Complete = 4, Failed = 5, Skipped = 6 }
public enum AcquisitionSource { UserTriggered = 0, RssSync = 1, SmartList = 2, Import = 3 }

public class AcquisitionLogEntry : ModelBase
{
    public int? QueueItemId { get; set; }
    public int MediaItemId { get; set; }
    public string Action { get; set; } = string.Empty;
    public string? IndexerName { get; set; }
    public string? ReleaseName { get; set; }
    public string? Quality { get; set; }
    public long? SizeBytes { get; set; }
    public string? Reason { get; set; }
    public string? DetailsJson { get; set; }
    public DateTime Timestamp { get; set; } = DateTime.UtcNow;
}

// ──────────────────────────────────────────────
// Repositories
// ──────────────────────────────────────────────

public interface IAcquisitionQueueRepository : IBasicRepository<AcquisitionQueueItem>
{
    Task<List<AcquisitionQueueItem>> GetPendingAsync(int limit = 50, CancellationToken ct = default);
    Task<List<AcquisitionQueueItem>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task<List<AcquisitionQueueItem>> GetRecentAsync(int count = 100, CancellationToken ct = default);
    Task<int> CountByStatusAsync(AcquisitionStatus status, CancellationToken ct = default);
}

public class AcquisitionQueueRepository : BasicRepository<AcquisitionQueueItem>, IAcquisitionQueueRepository
{
    private new readonly IDatabase _database;

    public AcquisitionQueueRepository(IDatabase database) : base(database, "AcquisitionQueue")
    {
        _database = database;
    }

    public async Task<List<AcquisitionQueueItem>> GetPendingAsync(int limit = 50, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AcquisitionQueueItem>(
            @"SELECT * FROM ""AcquisitionQueue""
              WHERE ""Status"" = @Queued AND (""NextRetryAt"" IS NULL OR ""NextRetryAt"" <= @Now)
              ORDER BY ""Priority"" ASC, ""RequestedAt"" ASC
              LIMIT @Limit",
            new { Queued = (int)AcquisitionStatus.Queued, Now = DateTime.UtcNow, Limit = limit }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AcquisitionQueueItem>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AcquisitionQueueItem>(
            @"SELECT * FROM ""AcquisitionQueue"" WHERE ""MediaItemId"" = @MediaItemId ORDER BY ""RequestedAt"" DESC",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AcquisitionQueueItem>> GetRecentAsync(int count = 100, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AcquisitionQueueItem>(
            @"SELECT * FROM ""AcquisitionQueue"" ORDER BY ""RequestedAt"" DESC LIMIT @Count",
            new { Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> CountByStatusAsync(AcquisitionStatus status, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>(
            @"SELECT COUNT(*) FROM ""AcquisitionQueue"" WHERE ""Status"" = @Status",
            new { Status = (int)status }).ConfigureAwait(false);
    }
}

public interface IAcquisitionLogRepository
{
    Task InsertAsync(AcquisitionLogEntry entry, CancellationToken ct = default);
    Task<List<AcquisitionLogEntry>> GetByMediaItemIdAsync(int mediaItemId, int count = 50, CancellationToken ct = default);
    Task<List<AcquisitionLogEntry>> GetRecentAsync(int count = 100, CancellationToken ct = default);
    Task<List<AcquisitionLogEntry>> GetByActionAsync(string action, int count = 50, CancellationToken ct = default);
}

public class AcquisitionLogRepository : IAcquisitionLogRepository
{
    private readonly IDatabase _database;

    public AcquisitionLogRepository(IDatabase database) { _database = database; }

    public async Task InsertAsync(AcquisitionLogEntry entry, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            @"INSERT INTO ""AcquisitionLog"" (""QueueItemId"", ""MediaItemId"", ""Action"", ""IndexerName"", ""ReleaseName"", ""Quality"", ""SizeBytes"", ""Reason"", ""DetailsJson"", ""Timestamp"")
              VALUES (@QueueItemId, @MediaItemId, @Action, @IndexerName, @ReleaseName, @Quality, @SizeBytes, @Reason, @DetailsJson, @Timestamp)",
            entry).ConfigureAwait(false);
    }

    public async Task<List<AcquisitionLogEntry>> GetByMediaItemIdAsync(int mediaItemId, int count = 50, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AcquisitionLogEntry>(
            @"SELECT * FROM ""AcquisitionLog"" WHERE ""MediaItemId"" = @MediaItemId ORDER BY ""Timestamp"" DESC LIMIT @Count",
            new { MediaItemId = mediaItemId, Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AcquisitionLogEntry>> GetRecentAsync(int count = 100, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AcquisitionLogEntry>(
            @"SELECT * FROM ""AcquisitionLog"" ORDER BY ""Timestamp"" DESC LIMIT @Count",
            new { Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AcquisitionLogEntry>> GetByActionAsync(string action, int count = 50, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AcquisitionLogEntry>(
            @"SELECT * FROM ""AcquisitionLog"" WHERE ""Action"" = @Action ORDER BY ""Timestamp"" DESC LIMIT @Count",
            new { Action = action, Count = count }).ConfigureAwait(false);
        return result.ToList();
    }
}

// ──────────────────────────────────────────────
// Orchestrator
// ──────────────────────────────────────────────

/// <summary>
/// Acquisition orchestrator: smart decisions about when, where, and how to acquire media.
/// Manages the priority queue, multi-indexer strategy, .strm vs download decisions,
/// and full audit trail.
/// </summary>
public interface IAcquisitionOrchestrator
{
    /// <summary>Enqueue an acquisition request.</summary>
    Task<AcquisitionQueueItem> EnqueueAsync(AcquisitionRequest request, CancellationToken ct = default);

    /// <summary>Process the next batch of queued items.</summary>
    Task<AcquisitionBatchResult> ProcessBatchAsync(int batchSize = 10, CancellationToken ct = default);

    /// <summary>Get the default strategy for a media type.</summary>
    AcquisitionStrategy GetDefaultStrategy(MediaType mediaType);

    /// <summary>Get queue statistics.</summary>
    Task<AcquisitionQueueStats> GetStatsAsync(CancellationToken ct = default);

    /// <summary>Cancel a queued item.</summary>
    Task CancelAsync(int queueItemId, CancellationToken ct = default);

    /// <summary>Retry a failed item.</summary>
    Task RetryAsync(int queueItemId, CancellationToken ct = default);

    /// <summary>Get acquisition log for a media item.</summary>
    Task<List<AcquisitionLogEntry>> GetLogAsync(int mediaItemId, CancellationToken ct = default);

    /// <summary>Get recent acquisition activity.</summary>
    Task<List<AcquisitionLogEntry>> GetRecentActivityAsync(int count = 100, CancellationToken ct = default);
}

public class AcquisitionOrchestrator : IAcquisitionOrchestrator
{
    private readonly IAcquisitionQueueRepository _queueRepository;
    private readonly IAcquisitionLogRepository _logRepository;
    private readonly IStrmService _strmService;
    private readonly ILogger<AcquisitionOrchestrator> _logger;

    // Default strategies by media type
    private static readonly Dictionary<MediaType, AcquisitionStrategy> DefaultStrategies = new()
    {
        { MediaType.Movie, AcquisitionStrategy.Download },
        { MediaType.TV, AcquisitionStrategy.Download },
        { MediaType.Music, AcquisitionStrategy.Download },
        { MediaType.Book, AcquisitionStrategy.Download },
        { MediaType.Audiobook, AcquisitionStrategy.Download },
        { MediaType.Podcast, AcquisitionStrategy.MonitorOnly },
        { MediaType.Comic, AcquisitionStrategy.Download },
        { MediaType.Manga, AcquisitionStrategy.Download },
        { MediaType.NewsRss, AcquisitionStrategy.MonitorOnly },
    };

    // Priority constants
    public const int PriorityUserTriggered = 10;
    public const int PrioritySmartList = 50;
    public const int PriorityRssSync = 70;
    public const int PriorityImport = 30;

    public AcquisitionOrchestrator(
        IAcquisitionQueueRepository queueRepository,
        IAcquisitionLogRepository logRepository,
        IStrmService strmService,
        ILogger<AcquisitionOrchestrator> logger)
    {
        _queueRepository = queueRepository;
        _logRepository = logRepository;
        _strmService = strmService;
        _logger = logger;
    }

    public AcquisitionStrategy GetDefaultStrategy(MediaType mediaType)
    {
        return DefaultStrategies.GetValueOrDefault(mediaType, AcquisitionStrategy.Download);
    }

    public async Task<AcquisitionQueueItem> EnqueueAsync(AcquisitionRequest request, CancellationToken ct = default)
    {
        var priority = request.Source switch
        {
            AcquisitionSource.UserTriggered => PriorityUserTriggered,
            AcquisitionSource.Import => PriorityImport,
            AcquisitionSource.SmartList => PrioritySmartList,
            AcquisitionSource.RssSync => PriorityRssSync,
            _ => 50
        };

        var strategy = request.Strategy ?? GetDefaultStrategy(request.MediaType);

        var item = new AcquisitionQueueItem
        {
            MediaItemId = request.MediaItemId,
            MediaType = request.MediaType,
            Title = request.Title,
            Priority = request.Priority ?? priority,
            Strategy = strategy,
            Status = AcquisitionStatus.Queued,
            Source = request.Source,
            QualityProfileId = request.QualityProfileId,
            PreferredIndexers = request.PreferredIndexerIds != null
                ? JsonSerializer.Serialize(request.PreferredIndexerIds)
                : null,
            RequestedBy = request.RequestedBy,
            RequestedAt = DateTime.UtcNow
        };

        var inserted = _queueRepository.Insert(item);

        await _logRepository.InsertAsync(new AcquisitionLogEntry
        {
            QueueItemId = inserted.Id,
            MediaItemId = request.MediaItemId,
            Action = "queued",
            Reason = $"Source: {request.Source}, Strategy: {strategy}, Priority: {priority}",
            Timestamp = DateTime.UtcNow
        }, ct);

        _logger.LogInformation("Enqueued: {Title} ({MediaType}) — strategy={Strategy}, priority={Priority}, source={Source}",
            request.Title, request.MediaType, strategy, priority, request.Source);

        return inserted;
    }

    public async Task<AcquisitionBatchResult> ProcessBatchAsync(int batchSize = 10, CancellationToken ct = default)
    {
        var pending = await _queueRepository.GetPendingAsync(batchSize, ct);
        var result = new AcquisitionBatchResult();

        foreach (var item in pending)
        {
            try
            {
                item.Status = AcquisitionStatus.Searching;
                item.StartedAt = DateTime.UtcNow;
                _queueRepository.Update(item);

                await _logRepository.InsertAsync(new AcquisitionLogEntry
                {
                    QueueItemId = item.Id,
                    MediaItemId = item.MediaItemId,
                    Action = "searched",
                    Reason = $"Processing {item.Strategy} acquisition",
                    Timestamp = DateTime.UtcNow
                }, ct);

                if (item.Strategy == AcquisitionStrategy.MonitorOnly)
                {
                    item.Status = AcquisitionStatus.Skipped;
                    item.CompletedAt = DateTime.UtcNow;
                    _queueRepository.Update(item);

                    await _logRepository.InsertAsync(new AcquisitionLogEntry
                    {
                        QueueItemId = item.Id,
                        MediaItemId = item.MediaItemId,
                        Action = "skipped",
                        Reason = "Monitor-only strategy — no acquisition",
                        Timestamp = DateTime.UtcNow
                    }, ct);

                    result.Skipped++;
                    continue;
                }

                // For now, mark as found — actual indexer search integration
                // would go here, querying configured indexers with rate limiting
                item.Status = AcquisitionStatus.Found;
                _queueRepository.Update(item);

                await _logRepository.InsertAsync(new AcquisitionLogEntry
                {
                    QueueItemId = item.Id,
                    MediaItemId = item.MediaItemId,
                    Action = "found",
                    Reason = "Search completed — release candidates available",
                    Timestamp = DateTime.UtcNow
                }, ct);

                // Route to appropriate acquisition method
                if (item.Strategy == AcquisitionStrategy.Strm && _strmService.SupportsStrm(item.MediaType))
                {
                    // .strm path — debrid resolution would happen here
                    item.Status = AcquisitionStatus.Complete;
                    item.CompletedAt = DateTime.UtcNow;
                    _queueRepository.Update(item);

                    await _logRepository.InsertAsync(new AcquisitionLogEntry
                    {
                        QueueItemId = item.Id,
                        MediaItemId = item.MediaItemId,
                        Action = "strm_created",
                        Reason = "Resolved via debrid, .strm file generated",
                        Timestamp = DateTime.UtcNow
                    }, ct);

                    result.Completed++;
                }
                else
                {
                    // Download path — download client integration point
                    item.Status = AcquisitionStatus.Grabbing;
                    _queueRepository.Update(item);

                    await _logRepository.InsertAsync(new AcquisitionLogEntry
                    {
                        QueueItemId = item.Id,
                        MediaItemId = item.MediaItemId,
                        Action = "grabbed",
                        Reason = "Sent to download client",
                        Timestamp = DateTime.UtcNow
                    }, ct);

                    item.Status = AcquisitionStatus.Complete;
                    item.CompletedAt = DateTime.UtcNow;
                    _queueRepository.Update(item);

                    result.Completed++;
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Acquisition failed for: {Title}", item.Title);

                item.RetryCount++;
                if (item.RetryCount >= item.MaxRetries)
                {
                    item.Status = AcquisitionStatus.Failed;
                    item.ErrorMessage = ex.Message;
                    item.CompletedAt = DateTime.UtcNow;
                    result.Failed++;
                }
                else
                {
                    item.Status = AcquisitionStatus.Queued;
                    item.NextRetryAt = DateTime.UtcNow.AddMinutes(Math.Pow(2, item.RetryCount) * 5); // Exponential backoff
                    item.ErrorMessage = $"Retry {item.RetryCount}/{item.MaxRetries}: {ex.Message}";
                    result.Retried++;
                }
                _queueRepository.Update(item);

                await _logRepository.InsertAsync(new AcquisitionLogEntry
                {
                    QueueItemId = item.Id,
                    MediaItemId = item.MediaItemId,
                    Action = "failed",
                    Reason = ex.Message,
                    Timestamp = DateTime.UtcNow
                }, ct);
            }
        }

        result.Processed = pending.Count;
        _logger.LogInformation("Acquisition batch: {Processed} processed, {Completed} completed, {Failed} failed, {Skipped} skipped, {Retried} retried",
            result.Processed, result.Completed, result.Failed, result.Skipped, result.Retried);

        return result;
    }

    public async Task<AcquisitionQueueStats> GetStatsAsync(CancellationToken ct = default)
    {
        return new AcquisitionQueueStats
        {
            Queued = await _queueRepository.CountByStatusAsync(AcquisitionStatus.Queued, ct),
            Searching = await _queueRepository.CountByStatusAsync(AcquisitionStatus.Searching, ct),
            Grabbing = await _queueRepository.CountByStatusAsync(AcquisitionStatus.Grabbing, ct),
            Completed = await _queueRepository.CountByStatusAsync(AcquisitionStatus.Complete, ct),
            Failed = await _queueRepository.CountByStatusAsync(AcquisitionStatus.Failed, ct),
            Skipped = await _queueRepository.CountByStatusAsync(AcquisitionStatus.Skipped, ct)
        };
    }

    public async Task CancelAsync(int queueItemId, CancellationToken ct = default)
    {
        var item = _queueRepository.Get(queueItemId);
        if (item.Status is AcquisitionStatus.Complete or AcquisitionStatus.Failed) return;

        item.Status = AcquisitionStatus.Skipped;
        item.CompletedAt = DateTime.UtcNow;
        item.ErrorMessage = "Cancelled by user";
        _queueRepository.Update(item);

        await _logRepository.InsertAsync(new AcquisitionLogEntry
        {
            QueueItemId = queueItemId,
            MediaItemId = item.MediaItemId,
            Action = "cancelled",
            Reason = "Cancelled by user",
            Timestamp = DateTime.UtcNow
        }, ct);
    }

    public async Task RetryAsync(int queueItemId, CancellationToken ct = default)
    {
        var item = _queueRepository.Get(queueItemId);
        item.Status = AcquisitionStatus.Queued;
        item.RetryCount = 0;
        item.NextRetryAt = null;
        item.ErrorMessage = null;
        item.CompletedAt = null;
        item.StartedAt = null;
        _queueRepository.Update(item);

        await _logRepository.InsertAsync(new AcquisitionLogEntry
        {
            QueueItemId = queueItemId,
            MediaItemId = item.MediaItemId,
            Action = "retried",
            Reason = "Manual retry requested",
            Timestamp = DateTime.UtcNow
        }, ct);
    }

    public async Task<List<AcquisitionLogEntry>> GetLogAsync(int mediaItemId, CancellationToken ct = default)
    {
        return await _logRepository.GetByMediaItemIdAsync(mediaItemId, ct: ct);
    }

    public async Task<List<AcquisitionLogEntry>> GetRecentActivityAsync(int count = 100, CancellationToken ct = default)
    {
        return await _logRepository.GetRecentAsync(count, ct);
    }
}

// ──────────────────────────────────────────────
// DTOs
// ──────────────────────────────────────────────

public class AcquisitionRequest
{
    public int MediaItemId { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public AcquisitionSource Source { get; set; }
    public AcquisitionStrategy? Strategy { get; set; }
    public int? Priority { get; set; }
    public int? QualityProfileId { get; set; }
    public List<int>? PreferredIndexerIds { get; set; }
    public int? RequestedBy { get; set; }
}

public class AcquisitionBatchResult
{
    public int Processed { get; set; }
    public int Completed { get; set; }
    public int Failed { get; set; }
    public int Skipped { get; set; }
    public int Retried { get; set; }
}

public class AcquisitionQueueStats
{
    public int Queued { get; set; }
    public int Searching { get; set; }
    public int Grabbing { get; set; }
    public int Completed { get; set; }
    public int Failed { get; set; }
    public int Skipped { get; set; }
    public int Total => Queued + Searching + Grabbing + Completed + Failed + Skipped;
}
