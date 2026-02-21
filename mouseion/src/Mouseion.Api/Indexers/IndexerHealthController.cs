// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Indexers.RateLimiting;

namespace Mouseion.Api.Indexers;

[ApiController]
[Route("api/v3/indexer/health")]
[Authorize]
public class IndexerHealthController : ControllerBase
{
    private readonly IIndexerRateLimiter _rateLimiter;

    public IndexerHealthController(IIndexerRateLimiter rateLimiter)
    {
        _rateLimiter = rateLimiter;
    }

    /// <summary>
    /// Get rate limit health status for all configured indexers.
    /// Shows requests used/remaining per window, backoff state, errors.
    /// </summary>
    [HttpGet]
    public ActionResult<List<IndexerHealthStatus>> GetHealth()
    {
        return Ok(_rateLimiter.GetHealthStatus());
    }

    /// <summary>
    /// Configure rate limit for a specific indexer.
    /// </summary>
    [HttpPut("{indexerName}")]
    public ActionResult Configure(string indexerName, [FromBody] IndexerRateLimitConfigRequest request)
    {
        _rateLimiter.Configure(indexerName, request.MaxRequestsPerHour);
        return NoContent();
    }

    /// <summary>
    /// Clear backoff state for an indexer (manual recovery).
    /// </summary>
    [HttpPost("{indexerName}/clear-backoff")]
    public ActionResult ClearBackoff(string indexerName)
    {
        _rateLimiter.ClearBackoff(indexerName);
        return NoContent();
    }
}

public class IndexerRateLimitConfigRequest
{
    public int MaxRequestsPerHour { get; set; } = 100;
}
