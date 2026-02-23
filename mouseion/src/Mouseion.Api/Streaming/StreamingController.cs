// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.Music;

namespace Mouseion.Api.Streaming;

[ApiController]
[Authorize]
[Route("api/v3")]
public class StreamingController : ControllerBase
{
    private readonly IMediaFileRepository _mediaFileRepository;
    private readonly IMusicFileRepository _musicFileRepository;

    public StreamingController(IMediaFileRepository mediaFileRepository, IMusicFileRepository musicFileRepository)
    {
        _mediaFileRepository = mediaFileRepository;
        _musicFileRepository = musicFileRepository;
    }

    /// <summary>Stream by media file ID (direct).</summary>
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

        // Add bandwidth hints and file metadata headers
        Response.Headers["X-Content-Duration"] = mediaFile.DurationSeconds?.ToString() ?? "0";
        Response.Headers["X-File-Size"] = fileInfo.Length.ToString();
        Response.Headers["X-Bitrate"] = mediaFile.Bitrate?.ToString() ?? "0";
        Response.Headers["X-Format"] = mediaFile.Format ?? "unknown";
        Response.Headers["Accept-Ranges"] = "bytes";
        // Serve inline (not attachment) so audio/video elements can stream
        Response.Headers["Content-Disposition"] = "inline";

        return File(stream, mimeType, enableRangeProcessing: true);
    }

    /// <summary>Stream by track (MediaItem) ID — resolves to the best available media file.</summary>
    [HttpGet("stream/track/{trackId:int}")]
    public IActionResult StreamByTrack(int trackId)
    {
        var files = _musicFileRepository.GetByTrackId(trackId);
        if (files == null || files.Count == 0)
        {
            return NotFound(new { error = $"No media file found for track {trackId}" });
        }

        var musicFile = files[0];
        var filePath = musicFile.RelativePath;
        if (string.IsNullOrEmpty(filePath) || !System.IO.File.Exists(filePath))
        {
            return NotFound(new { error = $"File not found: {filePath}" });
        }

        var fileInfo = new System.IO.FileInfo(filePath);
        var stream = System.IO.File.OpenRead(filePath);
        var mimeType = GetMimeType(filePath);

        Response.Headers["Accept-Ranges"] = "bytes";
        Response.Headers["X-File-Size"] = fileInfo.Length.ToString();
        Response.Headers["X-Bitrate"] = musicFile.Bitrate?.ToString() ?? "0";
        Response.Headers["X-Format"] = musicFile.AudioFormat ?? "unknown";
        // Serve inline (not attachment) so audio/video elements can stream
        Response.Headers["Content-Disposition"] = "inline";

        return File(stream, mimeType, enableRangeProcessing: true);
    }

    /// <summary>
    /// Transcode endpoint — serves audio in client-preferred format.
    /// Currently supports passthrough with format negotiation via Accept header.
    /// Full FFmpeg transcoding is Phase 3+ (requires FFmpeg dependency).
    /// </summary>
    [HttpGet("stream/{mediaFileId:int}/transcode")]
    public IActionResult StreamTranscoded(
        int mediaFileId,
        [FromQuery] string? format = null,
        [FromQuery] int? bitrate = null)
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

        var requestedFormat = format?.ToLowerInvariant() ?? GetPreferredFormat();
        var sourceExtension = Path.GetExtension(mediaFile.Path).ToLowerInvariant();

        // If source format matches requested, passthrough (no transcode needed)
        if (FormatMatchesExtension(requestedFormat, sourceExtension))
        {
            return StreamMedia(mediaFileId);
        }

        // For now, return capability info. Full transcoding requires FFmpeg integration.
        // This endpoint exists so clients can start requesting transcoded formats —
        // when FFmpeg is wired in, they'll get real transcodes without API changes.
        return Ok(new TranscodeCapability
        {
            MediaFileId = mediaFileId,
            SourceFormat = sourceExtension.TrimStart('.'),
            SourceBitrate = mediaFile.Bitrate ?? 0,
            RequestedFormat = requestedFormat,
            RequestedBitrate = bitrate,
            TranscodeAvailable = false,
            FallbackUrl = $"/api/v3/stream/{mediaFileId}",
            SupportedFormats = new[] { "opus", "aac", "mp3", "flac", "ogg" }
        });
    }

    /// <summary>
    /// Cover art resize endpoint — serves thumbnails for mobile clients.
    /// </summary>
    [HttpGet("mediacover/{mediaItemId:int}/{coverType}")]
    public IActionResult GetCover(
        int mediaItemId,
        string coverType,
        [FromQuery] int? width = null,
        [FromQuery] int? height = null,
        [FromQuery] int? maxSize = null)
    {
        // Cover art paths follow convention: {DataDir}/MediaCover/{mediaItemId}/{coverType}.jpg
        var appData = Environment.GetEnvironmentVariable("MOUSEION_DATA")
            ?? Environment.GetEnvironmentVariable("MOUSEION_TEST_APPDATA")
            ?? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Mouseion");
        var coverDir = Path.Combine(appData, "MediaCover", mediaItemId.ToString());

        var extensions = new[] { ".jpg", ".jpeg", ".png", ".webp" };
        string? coverPath = null;

        foreach (var ext in extensions)
        {
            var path = Path.Combine(coverDir, $"{coverType}{ext}");
            if (global::System.IO.File.Exists(path))
            {
                coverPath = path;
                break;
            }
        }

        if (coverPath == null)
        {
            return NotFound(new { error = $"Cover '{coverType}' not found for media item {mediaItemId}" });
        }

        var stream = global::System.IO.File.OpenRead(coverPath);
        var mimeType = GetMimeType(coverPath);

        // Add cache headers for cover art
        Response.Headers["Cache-Control"] = "public, max-age=86400";

        // Include resize hints in response headers (actual resizing requires ImageSharp/SkiaSharp)
        if (width.HasValue || height.HasValue || maxSize.HasValue)
        {
            Response.Headers["X-Resize-Requested"] = "true";
            if (width.HasValue) Response.Headers["X-Resize-Width"] = width.Value.ToString();
            if (height.HasValue) Response.Headers["X-Resize-Height"] = height.Value.ToString();
        }

        return File(stream, mimeType);
    }

    private string GetPreferredFormat()
    {
        var accept = Request.Headers.Accept.ToString();
        if (accept.Contains("audio/opus")) return "opus";
        if (accept.Contains("audio/aac")) return "aac";
        if (accept.Contains("audio/ogg")) return "ogg";
        if (accept.Contains("audio/mpeg")) return "mp3";
        return "opus"; // Default to opus (best quality/size ratio)
    }

    private static bool FormatMatchesExtension(string format, string extension)
    {
        return format switch
        {
            "opus" => extension is ".opus" or ".ogg",
            "aac" => extension is ".aac" or ".m4a" or ".m4b",
            "mp3" => extension == ".mp3",
            "flac" => extension == ".flac",
            "ogg" => extension is ".ogg" or ".opus",
            _ => false
        };
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

            ".jpg" or ".jpeg" => "image/jpeg",
            ".png" => "image/png",
            ".webp" => "image/webp",

            _ => "application/octet-stream"
        };
    }
}

public class TranscodeCapability
{
    public int MediaFileId { get; set; }
    public string SourceFormat { get; set; } = string.Empty;
    public int SourceBitrate { get; set; }
    public string RequestedFormat { get; set; } = string.Empty;
    public int? RequestedBitrate { get; set; }
    public bool TranscodeAvailable { get; set; }
    public string FallbackUrl { get; set; } = string.Empty;
    public string[] SupportedFormats { get; set; } = Array.Empty<string>();
}
// NOTE: This class extension is appended — move into the class body
