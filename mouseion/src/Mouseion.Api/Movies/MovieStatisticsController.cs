// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Movies;

namespace Mouseion.Api.Movies;

[ApiController]
[Route("api/v3/movies/statistics")]
[Authorize]
public class MovieStatisticsController : ControllerBase
{
    private readonly IMovieStatisticsService _movieStatisticsService;

    public MovieStatisticsController(IMovieStatisticsService movieStatisticsService)
    {
        _movieStatisticsService = movieStatisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<MovieStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _movieStatisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }
}
