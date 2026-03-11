// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.MediaItems;

[ApiController]
[Route("api/v3/media/sync")]
[Authorize]
public class MediaSyncController : ControllerBase
{
    private readonly IMediaItemRepository _mediaItemRepository;

    public MediaSyncController(IMediaItemRepository mediaItemRepository)
    {
        _mediaItemRepository = mediaItemRepository;
    }

    [HttpGet]
    public async Task<ActionResult<List<MediaItemResource>>> GetModifiedSince(
        [FromQuery] DateTime? modifiedSince,
        [FromQuery] string? mediaType,
        CancellationToken ct = default)
    {
        if (!modifiedSince.HasValue)
        {
            return BadRequest(new { error = "modifiedSince query parameter is required" });
        }

        MediaType? parsedType = null;
        if (!string.IsNullOrWhiteSpace(mediaType))
        {
            if (!Enum.TryParse<MediaType>(mediaType, ignoreCase: true, out var type))
            {
                return BadRequest(new { error = $"Invalid mediaType: {mediaType}" });
            }
            parsedType = type;
        }

        var items = await _mediaItemRepository.GetModifiedSinceAsync(modifiedSince.Value, parsedType, ct).ConfigureAwait(false);
        return Ok(items.Select(ToResource).ToList());
    }

    private static MediaItemResource ToResource(MediaItemSummary item)
    {
        return new MediaItemResource
        {
            Id = item.Id,
            Title = item.Title,
            Year = item.Year,
            MediaType = item.MediaType.ToString(),
            Monitored = item.Monitored,
            QualityProfileId = item.QualityProfileId,
            Path = item.Path,
            Added = item.Added,
            LastModified = item.LastModified
        };
    }
}
