// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.TV;

namespace Mouseion.Api.TV;

[ApiController]
[Route("api/v3/series/statistics")]
[Authorize]
public class SeriesStatisticsController : ControllerBase
{
    private readonly ISeriesStatisticsService _seriesStatisticsService;

    public SeriesStatisticsController(ISeriesStatisticsService seriesStatisticsService)
    {
        _seriesStatisticsService = seriesStatisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<SeriesStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _seriesStatisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }
}
