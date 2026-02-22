// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Download.Acquisition;
using Mouseion.Core.Download.Strm;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.Acquisition;

[ApiController]
[Authorize]
[Route("api/v3/acquisition")]
public class AcquisitionController : ControllerBase
{
    private readonly IAcquisitionOrchestrator _orchestrator;
    private readonly IStrmService _strmService;
    private readonly IDebridServiceRepository _debridRepository;
    private readonly IStrmFileRepository _strmFileRepository;

    public AcquisitionController(
        IAcquisitionOrchestrator orchestrator,
        IStrmService strmService,
        IDebridServiceRepository debridRepository,
        IStrmFileRepository strmFileRepository)
    {
        _orchestrator = orchestrator;
        _strmService = strmService;
        _debridRepository = debridRepository;
        _strmFileRepository = strmFileRepository;
    }

    // ──────────────────────────────────────────────
    // Queue
    // ──────────────────────────────────────────────

    /// <summary>Enqueue a media item for acquisition.</summary>
    [HttpPost("enqueue")]
    public async Task<ActionResult<QueueItemResource>> Enqueue([FromBody][Required] EnqueueRequest request, CancellationToken ct)
    {
        var item = await _orchestrator.EnqueueAsync(new AcquisitionRequest
        {
            MediaItemId = request.MediaItemId,
            MediaType = request.MediaType,
            Title = request.Title,
            Source = AcquisitionSource.UserTriggered,
            Strategy = request.Strategy,
            Priority = request.Priority,
            QualityProfileId = request.QualityProfileId,
            PreferredIndexerIds = request.PreferredIndexerIds,
            RequestedBy = GetCurrentUserId()
        }, ct);

        return CreatedAtAction(nameof(GetQueueStats), null, ToResource(item));
    }

    /// <summary>Process the next batch of queued items.</summary>
    [HttpPost("process")]
    public async Task<ActionResult<AcquisitionBatchResult>> ProcessBatch([FromQuery] int batchSize = 10, CancellationToken ct = default)
    {
        var result = await _orchestrator.ProcessBatchAsync(batchSize, ct);
        return Ok(result);
    }

    /// <summary>Get queue statistics.</summary>
    [HttpGet("queue/stats")]
    public async Task<ActionResult<AcquisitionQueueStats>> GetQueueStats(CancellationToken ct)
    {
        var stats = await _orchestrator.GetStatsAsync(ct);
        return Ok(stats);
    }

    /// <summary>Cancel a queued item.</summary>
    [HttpDelete("queue/{id:int}")]
    public async Task<ActionResult> Cancel(int id, CancellationToken ct)
    {
        await _orchestrator.CancelAsync(id, ct);
        return NoContent();
    }

    /// <summary>Retry a failed item.</summary>
    [HttpPost("queue/{id:int}/retry")]
    public async Task<ActionResult> Retry(int id, CancellationToken ct)
    {
        await _orchestrator.RetryAsync(id, ct);
        return NoContent();
    }

    /// <summary>Get default acquisition strategy for a media type.</summary>
    [HttpGet("strategy/{mediaType}")]
    public ActionResult<StrategyResource> GetStrategy(string mediaType)
    {
        if (!Enum.TryParse<MediaType>(mediaType, ignoreCase: true, out var mt))
        {
            return BadRequest(new { error = $"Unknown media type: {mediaType}" });
        }

        return Ok(new StrategyResource
        {
            MediaType = mt.ToString(),
            Strategy = _orchestrator.GetDefaultStrategy(mt).ToString(),
            SupportsStrm = _strmService.SupportsStrm(mt)
        });
    }

    // ──────────────────────────────────────────────
    // Acquisition Log
    // ──────────────────────────────────────────────

    /// <summary>Get acquisition log for a media item.</summary>
    [HttpGet("log/{mediaItemId:int}")]
    public async Task<ActionResult<List<AcquisitionLogResource>>> GetLog(int mediaItemId, CancellationToken ct)
    {
        var log = await _orchestrator.GetLogAsync(mediaItemId, ct);
        return Ok(log.Select(ToResource).ToList());
    }

    /// <summary>Get recent acquisition activity.</summary>
    [HttpGet("log")]
    public async Task<ActionResult<List<AcquisitionLogResource>>> GetRecentActivity([FromQuery] int count = 100, CancellationToken ct = default)
    {
        var log = await _orchestrator.GetRecentActivityAsync(count, ct);
        return Ok(log.Select(ToResource).ToList());
    }

    // ──────────────────────────────────────────────
    // Debrid Services
    // ──────────────────────────────────────────────

    /// <summary>List configured debrid services.</summary>
    [HttpGet("debrid")]
    public ActionResult<List<DebridServiceResource>> GetDebridServices()
    {
        var services = _debridRepository.All();
        return Ok(services.Select(s => new DebridServiceResource
        {
            Id = s.Id,
            Name = s.Name,
            Provider = s.Provider.ToString(),
            Enabled = s.Enabled,
            Priority = s.Priority,
            BandwidthLimitGb = s.BandwidthLimitGb,
            BandwidthUsedGb = s.BandwidthUsedGb,
            LastChecked = s.LastChecked
        }).ToList());
    }

    /// <summary>Add a debrid service.</summary>
    [HttpPost("debrid")]
    public ActionResult<DebridServiceResource> AddDebridService([FromBody][Required] AddDebridServiceRequest request)
    {
        if (!Enum.TryParse<DebridProvider>(request.Provider, ignoreCase: true, out var provider))
        {
            return BadRequest(new { error = $"Unknown provider: {request.Provider}. Supported: RealDebrid, AllDebrid, Premiumize" });
        }

        var service = new DebridServiceDefinition
        {
            Name = request.Name,
            Provider = provider,
            ApiKey = request.ApiKey,
            Enabled = true,
            Priority = request.Priority ?? 0,
            BandwidthLimitGb = request.BandwidthLimitGb,
            CreatedAt = DateTime.UtcNow
        };

        var inserted = _debridRepository.Insert(service);

        return CreatedAtAction(nameof(GetDebridServices), null, new DebridServiceResource
        {
            Id = inserted.Id,
            Name = inserted.Name,
            Provider = inserted.Provider.ToString(),
            Enabled = inserted.Enabled,
            Priority = inserted.Priority,
            BandwidthLimitGb = inserted.BandwidthLimitGb
        });
    }

    /// <summary>Delete a debrid service.</summary>
    [HttpDelete("debrid/{id:int}")]
    public ActionResult DeleteDebridService(int id)
    {
        _debridRepository.Delete(id);
        return NoContent();
    }

    // ──────────────────────────────────────────────
    // .strm Management
    // ──────────────────────────────────────────────

    /// <summary>Verify all .strm files for validity.</summary>
    [HttpPost("strm/verify")]
    public async Task<ActionResult<StrmVerificationResult>> VerifyStrm(CancellationToken ct)
    {
        var result = await _strmService.VerifyAllAsync(ct);
        return Ok(result);
    }

    /// <summary>Get .strm file for a media item.</summary>
    [HttpGet("strm/{mediaItemId:int}")]
    public async Task<ActionResult> GetStrm(int mediaItemId, CancellationToken ct)
    {
        var strm = await _strmFileRepository.GetByMediaItemIdAsync(mediaItemId, ct);
        if (strm == null) return NotFound();

        return Ok(new StrmFileResource
        {
            Id = strm.Id,
            MediaItemId = strm.MediaItemId,
            FilePath = strm.FilePath,
            Quality = strm.Quality,
            SizeBytes = strm.SizeBytes,
            IsValid = strm.IsValid,
            LastVerified = strm.LastVerified,
            ExpiresAt = strm.ExpiresAt,
            CreatedAt = strm.CreatedAt
        });
    }

    /// <summary>Delete a .strm file.</summary>
    [HttpDelete("strm/{id:int}")]
    public async Task<ActionResult> DeleteStrm(int id, CancellationToken ct)
    {
        await _strmService.DeleteStrmAsync(id, ct);
        return NoContent();
    }

    // ──────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────

    private int GetCurrentUserId()
    {
        var claim = User.FindFirst("userId")?.Value ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return int.TryParse(claim, out var id) ? id : 1;
    }

    private static QueueItemResource ToResource(AcquisitionQueueItem item) => new()
    {
        Id = item.Id,
        MediaItemId = item.MediaItemId,
        MediaType = item.MediaType.ToString(),
        Title = item.Title,
        Priority = item.Priority,
        Strategy = item.Strategy.ToString(),
        Status = item.Status.ToString(),
        Source = item.Source.ToString(),
        ErrorMessage = item.ErrorMessage,
        RetryCount = item.RetryCount,
        RequestedBy = item.RequestedBy,
        RequestedAt = item.RequestedAt,
        StartedAt = item.StartedAt,
        CompletedAt = item.CompletedAt
    };

    private static AcquisitionLogResource ToResource(AcquisitionLogEntry entry) => new()
    {
        Id = entry.Id,
        QueueItemId = entry.QueueItemId,
        MediaItemId = entry.MediaItemId,
        Action = entry.Action,
        IndexerName = entry.IndexerName,
        ReleaseName = entry.ReleaseName,
        Quality = entry.Quality,
        SizeBytes = entry.SizeBytes,
        Reason = entry.Reason,
        Timestamp = entry.Timestamp
    };
}

// ──────────────────────────────────────────────
// Resources
// ──────────────────────────────────────────────

public class EnqueueRequest
{
    public int MediaItemId { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public AcquisitionStrategy? Strategy { get; set; }
    public int? Priority { get; set; }
    public int? QualityProfileId { get; set; }
    public List<int>? PreferredIndexerIds { get; set; }
}

public class QueueItemResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string MediaType { get; set; } = string.Empty;
    public string Title { get; set; } = string.Empty;
    public int Priority { get; set; }
    public string Strategy { get; set; } = string.Empty;
    public string Status { get; set; } = string.Empty;
    public string Source { get; set; } = string.Empty;
    public string? ErrorMessage { get; set; }
    public int RetryCount { get; set; }
    public int? RequestedBy { get; set; }
    public DateTime RequestedAt { get; set; }
    public DateTime? StartedAt { get; set; }
    public DateTime? CompletedAt { get; set; }
}

public class AcquisitionLogResource
{
    public int Id { get; set; }
    public int? QueueItemId { get; set; }
    public int MediaItemId { get; set; }
    public string Action { get; set; } = string.Empty;
    public string? IndexerName { get; set; }
    public string? ReleaseName { get; set; }
    public string? Quality { get; set; }
    public long? SizeBytes { get; set; }
    public string? Reason { get; set; }
    public DateTime Timestamp { get; set; }
}

public class StrategyResource
{
    public string MediaType { get; set; } = string.Empty;
    public string Strategy { get; set; } = string.Empty;
    public bool SupportsStrm { get; set; }
}

public class DebridServiceResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string Provider { get; set; } = string.Empty;
    public bool Enabled { get; set; }
    public int Priority { get; set; }
    public int? BandwidthLimitGb { get; set; }
    public decimal? BandwidthUsedGb { get; set; }
    public DateTime? LastChecked { get; set; }
}

public class AddDebridServiceRequest
{
    public string Name { get; set; } = string.Empty;
    public string Provider { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public int? Priority { get; set; }
    public int? BandwidthLimitGb { get; set; }
}

public class StrmFileResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string FilePath { get; set; } = string.Empty;
    public string? Quality { get; set; }
    public long? SizeBytes { get; set; }
    public bool IsValid { get; set; }
    public DateTime? LastVerified { get; set; }
    public DateTime? ExpiresAt { get; set; }
    public DateTime CreatedAt { get; set; }
}
