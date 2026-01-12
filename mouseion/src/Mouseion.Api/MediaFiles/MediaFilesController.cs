// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Common.Crypto;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Api.MediaFiles;

[ApiController]
[Route("api/v3/mediafiles")]
[Authorize]
public class MediaFilesController : ControllerBase
{
    private readonly IMediaFileRepository _mediaFileRepository;
    private readonly IHashProvider _hashProvider;

    public MediaFilesController(
        IMediaFileRepository mediaFileRepository,
        IHashProvider hashProvider)
    {
        _mediaFileRepository = mediaFileRepository;
        _hashProvider = hashProvider;
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<MediaFileResource>> GetById(int id, CancellationToken ct = default)
    {
        var mediaFile = await _mediaFileRepository.FindAsync(id, ct);

        if (mediaFile == null)
        {
            return NotFound(new { error = $"MediaFile with ID {id} not found" });
        }

        return Ok(MapToResource(mediaFile));
    }

    [HttpGet]
    public async Task<ActionResult<List<MediaFileResource>>> GetByMediaItem(
        [FromQuery] int? mediaItemId,
        CancellationToken ct = default)
    {
        if (mediaItemId == null)
        {
            return BadRequest(new { error = "mediaItemId query parameter is required" });
        }

        var mediaFiles = await _mediaFileRepository.GetByMediaItemIdAsync(mediaItemId.Value, ct);
        return Ok(mediaFiles.Select(MapToResource).ToList());
    }

    [HttpPost("{id:int}/hash")]
    public async Task<ActionResult<MediaFileResource>> ComputeHash(int id, CancellationToken ct = default)
    {
        var mediaFile = await _mediaFileRepository.FindAsync(id, ct);

        if (mediaFile == null)
        {
            return NotFound(new { error = $"MediaFile with ID {id} not found" });
        }

        if (!global::System.IO.File.Exists(mediaFile.Path))
        {
            return BadRequest(new { error = $"File not found at path: {mediaFile.Path}" });
        }

        mediaFile.FileHash = _hashProvider.ComputeHashString(mediaFile.Path);
        await _mediaFileRepository.UpdateAsync(mediaFile, ct);

        return Ok(MapToResource(mediaFile));
    }

    private static MediaFileResource MapToResource(MediaFile mediaFile)
    {
        return new MediaFileResource
        {
            Id = mediaFile.Id,
            MediaItemId = mediaFile.MediaItemId,
            MediaType = mediaFile.MediaType.ToString(),
            Path = mediaFile.Path,
            RelativePath = mediaFile.RelativePath,
            Size = mediaFile.Size,
            DateAdded = mediaFile.DateAdded,
            DurationSeconds = mediaFile.DurationSeconds,
            Bitrate = mediaFile.Bitrate,
            SampleRate = mediaFile.SampleRate,
            Channels = mediaFile.Channels,
            Format = mediaFile.Format,
            Quality = mediaFile.Quality,
            FileHash = mediaFile.FileHash
        };
    }
}

public class MediaFileResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string MediaType { get; set; } = null!;
    public string Path { get; set; } = null!;
    public string? RelativePath { get; set; }
    public long Size { get; set; }
    public DateTime DateAdded { get; set; }
    public int? DurationSeconds { get; set; }
    public int? Bitrate { get; set; }
    public int? SampleRate { get; set; }
    public int? Channels { get; set; }
    public string? Format { get; set; }
    public string? Quality { get; set; }
    public string? FileHash { get; set; }
}
