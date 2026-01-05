// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Api.Streaming;

[ApiController]
[Route("api/v3")]
public class StreamingController : ControllerBase
{
    private readonly IMediaFileRepository _mediaFileRepository;

    public StreamingController(IMediaFileRepository mediaFileRepository)
    {
        _mediaFileRepository = mediaFileRepository;
    }

    [HttpGet("stream/{mediaFileId:int}")]
    public IActionResult StreamMedia(int mediaFileId)
    {
        var mediaFile = _mediaFileRepository.Find(mediaFileId);
        if (mediaFile == null)
        {
            return NotFound(new { error = $"MediaFile {mediaFileId} not found" });
        }

        if (!global::System.IO.File.Exists(mediaFile.Path))
        {
            return NotFound(new { error = $"File not found: {mediaFile.Path}" });
        }

        var fileInfo = new global::System.IO.FileInfo(mediaFile.Path);
        var stream = global::System.IO.File.OpenRead(mediaFile.Path);

        var mimeType = GetMimeType(mediaFile.Path);

        return File(stream, mimeType, fileInfo.Name, enableRangeProcessing: true);
    }

    private static string GetMimeType(string filePath)
    {
        var extension = Path.GetExtension(filePath).ToLowerInvariant();

        return extension switch
        {
            ".m4b" => "audio/mp4",
            ".m4a" => "audio/mp4",
            ".mp3" => "audio/mpeg",
            ".flac" => "audio/flac",
            ".ogg" => "audio/ogg",
            ".opus" => "audio/opus",
            ".wav" => "audio/wav",
            ".aac" => "audio/aac",
            ".wma" => "audio/x-ms-wma",

            ".mp4" => "video/mp4",
            ".mkv" => "video/x-matroska",
            ".avi" => "video/x-msvideo",
            ".webm" => "video/webm",

            _ => "application/octet-stream"
        };
    }
}
