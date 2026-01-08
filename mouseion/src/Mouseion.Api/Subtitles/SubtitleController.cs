// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Subtitles;

namespace Mouseion.Api.Subtitles;

[ApiController]
[Route("api/v3/subtitles")]
[Authorize]
public class SubtitleController : ControllerBase
{
    private readonly ISubtitleService _subtitleService;

    public SubtitleController(ISubtitleService subtitleService)
    {
        _subtitleService = subtitleService;
    }

    [HttpGet("search")]
    public async Task<ActionResult<List<SubtitleResource>>> SearchSubtitles(
        [FromQuery] int movieId,
        [FromQuery] string? language = null,
        CancellationToken ct = default)
    {
        if (movieId <= 0)
        {
            return BadRequest(new { error = "Movie ID is required" });
        }

        var results = await _subtitleService.SearchForMovieAsync(movieId, language, ct).ConfigureAwait(false);
        return Ok(results.Select(ToResource).ToList());
    }

    [HttpPost("download")]
    public async Task<ActionResult<SubtitleDownloadResult>> DownloadSubtitle(
        [FromBody][Required] SubtitleDownloadRequest request,
        CancellationToken ct = default)
    {
        if (request.FileId <= 0)
        {
            return BadRequest(new { error = "File ID is required" });
        }

        if (request.MovieId <= 0)
        {
            return BadRequest(new { error = "Movie ID is required" });
        }

        try
        {
            var path = await _subtitleService.DownloadSubtitleAsync(request.FileId, request.MovieId, ct).ConfigureAwait(false);
            return Ok(new SubtitleDownloadResult
            {
                Success = true,
                Path = path
            });
        }
        catch (ArgumentException ex)
        {
            return NotFound(new { error = ex.Message });
        }
        catch (InvalidOperationException ex)
        {
            return BadRequest(new { error = ex.Message });
        }
    }

    private static SubtitleResource ToResource(SubtitleSearchResult result)
    {
        return new SubtitleResource
        {
            FileId = result.FileId,
            FileName = result.FileName,
            Language = result.Language,
            DownloadCount = result.DownloadCount,
            Rating = result.Rating,
            MovieHashMatch = result.MovieHash,
            Uploader = result.Uploader
        };
    }
}

public class SubtitleResource
{
    public int FileId { get; set; }
    public string FileName { get; set; } = string.Empty;
    public string Language { get; set; } = string.Empty;
    public int DownloadCount { get; set; }
    public float Rating { get; set; }
    public bool MovieHashMatch { get; set; }
    public string? Uploader { get; set; }
}

public class SubtitleDownloadRequest
{
    public int FileId { get; set; }
    public int MovieId { get; set; }
}

public class SubtitleDownloadResult
{
    public bool Success { get; set; }
    public string Path { get; set; } = string.Empty;
}
