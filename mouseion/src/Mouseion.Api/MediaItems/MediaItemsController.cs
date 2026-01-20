// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.MediaItems;

[ApiController]
[Route("api/v3/media")]
[Authorize]
public class MediaItemsController : ControllerBase
{
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly IMediaFileRepository _mediaFileRepository;

    public MediaItemsController(
        IMediaItemRepository mediaItemRepository,
        IMediaFileRepository mediaFileRepository)
    {
        _mediaItemRepository = mediaItemRepository;
        _mediaFileRepository = mediaFileRepository;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<MediaItemResource>>> GetAll(
        [FromQuery] string? mediaType,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        MediaType? parsedType = null;
        if (!string.IsNullOrWhiteSpace(mediaType))
        {
            if (!Enum.TryParse<MediaType>(mediaType, ignoreCase: true, out var type))
            {
                return BadRequest(new { error = $"Invalid mediaType: {mediaType}. Valid values: {string.Join(", ", Enum.GetNames<MediaType>())}" });
            }
            parsedType = type;
        }

        var totalCount = await _mediaItemRepository.CountAsync(parsedType, ct).ConfigureAwait(false);
        var items = await _mediaItemRepository.GetPageAsync(page, pageSize, parsedType, ct).ConfigureAwait(false);

        return Ok(new PagedResult<MediaItemResource>
        {
            Items = items.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<MediaItemDetailResource>> GetById(int id, CancellationToken ct = default)
    {
        var mediaItem = await _mediaItemRepository.FindByIdAsync(id, ct).ConfigureAwait(false);
        if (mediaItem == null)
        {
            return NotFound(new { error = $"MediaItem with ID {id} not found" });
        }

        var files = await _mediaFileRepository.GetByMediaItemIdAsync(id, ct).ConfigureAwait(false);

        return Ok(new MediaItemDetailResource
        {
            Id = mediaItem.Id,
            Title = mediaItem.GetTitle(),
            Year = mediaItem.GetYear(),
            MediaType = mediaItem.MediaType.ToString(),
            Monitored = mediaItem.Monitored,
            QualityProfileId = mediaItem.QualityProfileId,
            Path = mediaItem.Path,
            Added = mediaItem.Added,
            Tags = mediaItem.Tags.ToList(),
            Files = files.Select(f => new MediaFileResource
            {
                Id = f.Id,
                MediaItemId = f.MediaItemId,
                Path = f.Path,
                RelativePath = f.RelativePath,
                Size = f.Size,
                DateAdded = f.DateAdded,
                Format = f.Format,
                Quality = f.Quality,
                FileHash = f.FileHash
            }).ToList()
        });
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> Delete(int id, CancellationToken ct = default)
    {
        var mediaItem = await _mediaItemRepository.FindByIdAsync(id, ct).ConfigureAwait(false);
        if (mediaItem == null)
        {
            return NotFound(new { error = $"MediaItem with ID {id} not found" });
        }

        await _mediaItemRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
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

public class MediaItemResource
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public string MediaType { get; set; } = string.Empty;
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string Path { get; set; } = string.Empty;
    public DateTime Added { get; set; }
    public DateTime? LastModified { get; set; }
}

public class MediaItemDetailResource : MediaItemResource
{
    public List<int> Tags { get; set; } = new();
    public List<MediaFileResource> Files { get; set; } = new();
}

public class MediaFileResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string Path { get; set; } = string.Empty;
    public string? RelativePath { get; set; }
    public long Size { get; set; }
    public DateTime DateAdded { get; set; }
    public string? Format { get; set; }
    public string? Quality { get; set; }
    public string? FileHash { get; set; }
}
