// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Library;

namespace Mouseion.Api.Library;

[ApiController]
[Route("api/v3/library/statistics")]
[Authorize]
public class LibraryStatisticsController : ControllerBase
{
    private readonly IUnifiedLibraryStatisticsService _statisticsService;

    public LibraryStatisticsController(IUnifiedLibraryStatisticsService statisticsService)
    {
        _statisticsService = statisticsService;
    }

    /// <summary>
    /// Get unified statistics across all Phase 9 media types (News, Manga, Webcomics)
    /// </summary>
    [HttpGet]
    public async Task<ActionResult<UnifiedLibraryStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }
}
