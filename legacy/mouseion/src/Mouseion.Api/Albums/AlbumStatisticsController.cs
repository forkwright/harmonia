// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Music;

namespace Mouseion.Api.Albums;

[ApiController]
[Route("api/v3/albums/statistics")]
[Authorize]
public class AlbumStatisticsController : ControllerBase
{
    private readonly IAlbumStatisticsService _statisticsService;

    public AlbumStatisticsController(IAlbumStatisticsService statisticsService)
    {
        _statisticsService = statisticsService;
    }

    [HttpGet("{albumId:int}")]
    public async Task<ActionResult<AlbumStatistics>> GetStatistics(int albumId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(albumId, ct).ConfigureAwait(false);
        return Ok(stats);
    }
}
