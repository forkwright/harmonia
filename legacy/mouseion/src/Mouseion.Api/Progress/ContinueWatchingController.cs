// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Progress;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Api.Progress;

[ApiController]
[Route("api/v3/continue")]
[Authorize]
public class ContinueWatchingController : ControllerBase
{
    private readonly IMediaProgressRepository _progressRepository;
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly IMediaFileRepository _mediaFileRepository;

    public ContinueWatchingController(
        IMediaProgressRepository progressRepository,
        IMediaItemRepository mediaItemRepository,
        IMediaFileRepository mediaFileRepository)
    {
        _progressRepository = progressRepository;
        _mediaItemRepository = mediaItemRepository;
        _mediaFileRepository = mediaFileRepository;
    }

    [HttpGet]
    public async Task<ActionResult<List<ContinueResource>>> GetContinue(
        [FromQuery] int limit = 20,
        [FromQuery] string? mediaType = null,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var progressList = await _progressRepository.GetInProgressAsync(userId.ToString(), limit, ct).ConfigureAwait(false);
        var result = new List<ContinueResource>();

        foreach (var progress in progressList)
        {
            var mediaItem = await _mediaItemRepository.FindByIdAsync(progress.MediaItemId, ct).ConfigureAwait(false);
            if (mediaItem == null) continue;

            // Filter by media type if specified
            if (mediaType != null && !string.Equals(mediaItem.MediaType.ToString(), mediaType, StringComparison.OrdinalIgnoreCase))
                continue;

            var mediaFiles = await _mediaFileRepository.GetByMediaItemIdAsync(progress.MediaItemId, ct).ConfigureAwait(false);
            var primaryFile = mediaFiles.FirstOrDefault();

            result.Add(new ContinueResource
            {
                MediaItemId = progress.MediaItemId,
                Title = mediaItem.GetTitle(),
                MediaType = mediaItem.MediaType.ToString(),
                PositionMs = progress.PositionMs,
                TotalDurationMs = progress.TotalDurationMs,
                PercentComplete = progress.PercentComplete,
                LastPlayedAt = progress.LastPlayedAt,
                MediaFileId = primaryFile?.Id,
                CoverUrl = $"/api/v3/mediacover/{progress.MediaItemId}/poster"
            });
        }

        return Ok(result);
    }

    private int GetCurrentUserId()
    {
        var userIdClaim = User.FindFirst("userId")?.Value
            ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;

        return int.TryParse(userIdClaim, out var id) ? id : 1;
    }
}
