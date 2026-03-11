// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Audiobooks;

namespace Mouseion.Api.Audiobooks;

[ApiController]
[Route("api/v3/audiobooks/statistics")]
[Authorize]
public class AudiobookStatisticsController : ControllerBase
{
    private readonly IAudiobookStatisticsService _statisticsService;

    public AudiobookStatisticsController(IAudiobookStatisticsService statisticsService)
    {
        _statisticsService = statisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<AudiobookStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("author/{authorId:int}")]
    public async Task<ActionResult<AudiobookStatistics>> GetAuthorStatistics(int authorId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetAuthorStatisticsAsync(authorId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("series/{seriesId:int}")]
    public async Task<ActionResult<AudiobookStatistics>> GetSeriesStatistics(int seriesId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetSeriesStatisticsAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("narrator/{narrator}")]
    public async Task<ActionResult<AudiobookStatistics>> GetNarratorStatistics(string narrator, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetNarratorStatisticsAsync(narrator, ct).ConfigureAwait(false);
        return Ok(stats);
    }
}
