// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Api.Resources;
using Mouseion.Core.Filtering;
using Mouseion.Core.Library;
using Mouseion.Core.Music;

namespace Mouseion.Api.Library;

[ApiController]
[Route("api/v3/library")]
[Authorize]
public class LibraryController : ControllerBase
{
    private readonly ILibraryFilterService _filterService;
    private readonly IMusicFileRepository _musicFileRepository;

    public LibraryController(
        ILibraryFilterService filterService,
        IMusicFileRepository musicFileRepository)
    {
        _filterService = filterService;
        _musicFileRepository = musicFileRepository;
    }

    [HttpPost("filter")]
    public async Task<ActionResult<FilterPagedResult<TrackResource>>> FilterLibrary(
        [FromBody][Required] FilterRequest request,
        CancellationToken ct = default)
    {
        var result = await _filterService.FilterTracksAsync(request, ct).ConfigureAwait(false);
        var resources = await TrackResourceMapper.ToResourcesWithFilesAsync(
            result.Tracks, _musicFileRepository, ct).ConfigureAwait(false);

        return Ok(new FilterPagedResult<TrackResource>
        {
            Items = resources,
            Page = result.Page,
            PageSize = result.PageSize,
            TotalCount = result.TotalCount,
            Summary = result.Summary
        });
    }
}
