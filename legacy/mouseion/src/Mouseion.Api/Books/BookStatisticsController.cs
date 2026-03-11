// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Books;

namespace Mouseion.Api.Books;

[ApiController]
[Route("api/v3/books/statistics")]
[Authorize]
public class BookStatisticsController : ControllerBase
{
    private readonly IBookStatisticsService _statisticsService;

    public BookStatisticsController(IBookStatisticsService statisticsService)
    {
        _statisticsService = statisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<BookStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("author/{authorId:int}")]
    public async Task<ActionResult<BookStatistics>> GetAuthorStatistics(int authorId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetAuthorStatisticsAsync(authorId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("series/{seriesId:int}")]
    public async Task<ActionResult<BookStatistics>> GetSeriesStatistics(int seriesId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetSeriesStatisticsAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(stats);
    }
}
