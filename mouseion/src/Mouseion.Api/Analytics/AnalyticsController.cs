// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Analytics;

namespace Mouseion.Api.Analytics;

[ApiController]
[Route("api/v3/[controller]")]
public class AnalyticsController : ControllerBase
{
    private readonly IAnalyticsService _analyticsService;

    public AnalyticsController(IAnalyticsService analyticsService)
    {
        _analyticsService = analyticsService;
    }

    /// <summary>
    /// Get consumption statistics for a time period.
    /// </summary>
    /// <param name="period">Time period: 7d, 30d, 90d, 365d, all (default: 30d)</param>
    /// <param name="userId">User ID (default: "default")</param>
    [HttpGet("consumption")]
    public async Task<ActionResult<ConsumptionStats>> GetConsumptionStats(
        [FromQuery] string period = "30d",
        [FromQuery] string userId = "default",
        CancellationToken ct = default)
    {
        var dateRange = ParsePeriod(period);
        var stats = await _analyticsService.GetConsumptionStatsAsync(userId, dateRange, ct);
        return Ok(stats);
    }

    /// <summary>
    /// Get taste profile for a user (all-time analysis).
    /// </summary>
    /// <param name="userId">User ID (default: "default")</param>
    [HttpGet("taste")]
    public async Task<ActionResult<TasteProfile>> GetTasteProfile(
        [FromQuery] string userId = "default",
        CancellationToken ct = default)
    {
        var profile = await _analyticsService.GetTasteProfileAsync(userId, ct);
        return Ok(profile);
    }

    /// <summary>
    /// Get daily activity heatmap data for the specified period.
    /// Returns an array of { date, sessions, totalTimeMs } objects.
    /// </summary>
    [HttpGet("activity")]
    public async Task<ActionResult<List<DailyActivity>>> GetActivityHeatmap(
        [FromQuery] string period = "90d",
        [FromQuery] string userId = "default",
        CancellationToken ct = default)
    {
        var dateRange = ParsePeriod(period);
        var stats = await _analyticsService.GetConsumptionStatsAsync(userId, dateRange, ct);
        return Ok(stats.DailyActivity);
    }

    private static DateRange ParsePeriod(string period) => period.ToLowerInvariant() switch
    {
        "7d" => DateRange.Last7Days(),
        "30d" => DateRange.Last30Days(),
        "90d" => DateRange.Last90Days(),
        "365d" or "1y" => DateRange.Last365Days(),
        "all" => DateRange.AllTime(),
        _ => DateRange.Last30Days()
    };
}
