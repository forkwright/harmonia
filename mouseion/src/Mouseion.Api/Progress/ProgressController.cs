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
using Mouseion.Core.Progress;
using Mouseion.Core.MediaItems;

namespace Mouseion.Api.Progress;

[ApiController]
[Route("api/v3/progress")]
[Authorize]
public class ProgressController : ControllerBase
{
    private readonly IMediaProgressRepository _progressRepository;
    private readonly IMediaItemRepository _mediaItemRepository;

    public ProgressController(
        IMediaProgressRepository progressRepository,
        IMediaItemRepository mediaItemRepository)
    {
        _progressRepository = progressRepository;
        _mediaItemRepository = mediaItemRepository;
    }

    [HttpGet("{mediaItemId:int}")]
    public async Task<ActionResult<MediaProgressResource>> GetProgress(
        int mediaItemId,
        [FromQuery] string userId = "default",
        CancellationToken ct = default)
    {
        var progress = await _progressRepository.GetByMediaItemIdAsync(mediaItemId, userId, ct).ConfigureAwait(false);
        if (progress == null)
        {
            return NotFound(new { error = $"No progress found for media item {mediaItemId}" });
        }

        return Ok(ToResource(progress));
    }

    [HttpPost]
    public async Task<ActionResult<MediaProgressResource>> UpdateProgress(
        [FromBody][Required] UpdateProgressRequest request,
        CancellationToken ct = default)
    {
        var mediaItem = await _mediaItemRepository.FindByIdAsync(request.MediaItemId, ct).ConfigureAwait(false);
        if (mediaItem == null)
        {
            return NotFound(new { error = $"Media item {request.MediaItemId} not found" });
        }

        var progress = new MediaProgress
        {
            MediaItemId = request.MediaItemId,
            UserId = request.UserId ?? "default",
            PositionMs = request.PositionMs,
            TotalDurationMs = request.TotalDurationMs,
            PercentComplete = request.TotalDurationMs > 0
                ? Math.Round((decimal)request.PositionMs / request.TotalDurationMs * 100, 2)
                : 0,
            LastPlayedAt = DateTime.UtcNow,
            IsComplete = request.IsComplete
        };

        await _progressRepository.UpsertAsync(progress, ct).ConfigureAwait(false);

        return Ok(ToResource(progress));
    }

    [HttpDelete("{mediaItemId:int}")]
    public async Task<ActionResult> DeleteProgress(
        int mediaItemId,
        [FromQuery] string userId = "default",
        CancellationToken ct = default)
    {
        await _progressRepository.DeleteByMediaItemIdAsync(mediaItemId, userId, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static MediaProgressResource ToResource(MediaProgress progress)
    {
        return new MediaProgressResource
        {
            Id = progress.Id,
            MediaItemId = progress.MediaItemId,
            UserId = progress.UserId,
            PositionMs = progress.PositionMs,
            TotalDurationMs = progress.TotalDurationMs,
            PercentComplete = progress.PercentComplete,
            LastPlayedAt = progress.LastPlayedAt,
            IsComplete = progress.IsComplete,
            CreatedAt = progress.CreatedAt,
            UpdatedAt = progress.UpdatedAt
        };
    }
}

public class ContinueResource
{
    public int MediaItemId { get; set; }
    public string Title { get; set; } = string.Empty;
    public string MediaType { get; set; } = string.Empty;
    public long PositionMs { get; set; }
    public long TotalDurationMs { get; set; }
    public decimal PercentComplete { get; set; }
    public DateTime LastPlayedAt { get; set; }
    public int? MediaFileId { get; set; }
    public string CoverUrl { get; set; } = string.Empty;
}

public class MediaProgressResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string UserId { get; set; } = "default";
    public long PositionMs { get; set; }
    public long TotalDurationMs { get; set; }
    public decimal PercentComplete { get; set; }
    public DateTime LastPlayedAt { get; set; }
    public bool IsComplete { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class UpdateProgressRequest
{
    public int MediaItemId { get; set; }
    public string? UserId { get; set; }
    public long PositionMs { get; set; }
    public long TotalDurationMs { get; set; }
    public bool IsComplete { get; set; }
}
