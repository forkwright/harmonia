// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

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
        [FromQuery] string userId = "default",
        [FromQuery] int limit = 20,
        CancellationToken ct = default)
    {
        var progressList = await _progressRepository.GetInProgressAsync(userId, limit, ct).ConfigureAwait(false);
        var result = new List<ContinueResource>();

        foreach (var progress in progressList)
        {
            var mediaItem = await _mediaItemRepository.FindByIdAsync(progress.MediaItemId, ct).ConfigureAwait(false);
            if (mediaItem == null) continue;

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
}
