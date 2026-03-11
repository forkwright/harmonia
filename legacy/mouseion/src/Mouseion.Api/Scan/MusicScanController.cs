// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Hosting;
using Mouseion.Core.MediaFiles;
using Mouseion.Api.Library;

namespace Mouseion.Api.Scan;

[ApiController]
[Route("api/v3/scan/music")]
[Authorize]
public class MusicScanController : ControllerBase
{
    private readonly IMusicFileScanner _musicFileScanner;
    private readonly IMemoryCache _cache;
    private readonly IHostApplicationLifetime _appLifetime;
    private static ScanStatus? _currentScan;
    private static readonly object _lock = new();

    public MusicScanController(
        IMusicFileScanner musicFileScanner,
        IMemoryCache cache,
        IHostApplicationLifetime appLifetime)
    {
        _musicFileScanner = musicFileScanner;
        _cache = cache;
        _appLifetime = appLifetime;
    }

    /// <summary>Get current scan status.</summary>
    [HttpGet("status")]
    public ActionResult<ScanStatus> GetStatus()
    {
        lock (_lock)
        {
            return Ok(_currentScan ?? new ScanStatus { State = "idle" });
        }
    }

    [HttpPost("artist/{id:int}")]
    public async Task<ActionResult<ScanResultResource>> ScanArtist(int id, CancellationToken ct = default)
    {
        var result = await _musicFileScanner.ScanArtistAsync(id, ct).ConfigureAwait(false);

        if (!result.Success)
        {
            return BadRequest(new { error = result.Error });
        }

        FacetsController.InvalidateCache(_cache);
        return Ok(ToResource(result));
    }

    [HttpPost("album/{id:int}")]
    public async Task<ActionResult<ScanResultResource>> ScanAlbum(int id, CancellationToken ct = default)
    {
        var result = await _musicFileScanner.ScanAlbumAsync(id, ct).ConfigureAwait(false);

        if (!result.Success)
        {
            return BadRequest(new { error = result.Error });
        }

        FacetsController.InvalidateCache(_cache);
        return Ok(ToResource(result));
    }

    [HttpPost("rootfolder/{id:int}")]
    public ActionResult ScanRootFolder(int id)
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
            {
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            }

            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow, RootFolderId = id };
        }

        // Fire and forget — scan runs on background thread, not tied to HTTP request
        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _musicFileScanner.ScanRootFolderAsync(id, _appLifetime.ApplicationStopping).ConfigureAwait(false);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        RootFolderId = id,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
                FacetsController.InvalidateCache(_cache);
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Scan started", status = "scanning", pollUrl = "/api/v3/scan/music/status" });
    }

    [HttpPost("library")]
    public ActionResult ScanLibrary()
    {
        lock (_lock)
        {
            if (_currentScan?.State == "scanning")
            {
                return Conflict(new { error = "Scan already in progress", status = _currentScan });
            }

            _currentScan = new ScanStatus { State = "scanning", StartedAt = DateTime.UtcNow };
        }

        _ = Task.Run(async () =>
        {
            try
            {
                var result = await _musicFileScanner.ScanLibraryAsync(_appLifetime.ApplicationStopping).ConfigureAwait(false);
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = result.Success ? "completed" : "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        FilesFound = result.FilesFound,
                        FilesImported = result.FilesImported,
                        FilesRejected = result.FilesRejected,
                        Error = result.Error
                    };
                }
                FacetsController.InvalidateCache(_cache);
            }
            catch (Exception ex)
            {
                lock (_lock)
                {
                    _currentScan = new ScanStatus
                    {
                        State = "failed",
                        StartedAt = _currentScan?.StartedAt ?? DateTime.UtcNow,
                        CompletedAt = DateTime.UtcNow,
                        Error = ex.Message
                    };
                }
            }
        });

        return Accepted(new { message = "Library scan started", status = "scanning", pollUrl = "/api/v3/scan/music/status" });
    }

    private static ScanResultResource ToResource(ScanResult result)
    {
        return new ScanResultResource
        {
            FilesFound = result.FilesFound,
            FilesImported = result.FilesImported,
            FilesRejected = result.FilesRejected
        };
    }
}

public class ScanResultResource
{
    public int FilesFound { get; set; }
    public int FilesImported { get; set; }
    public int FilesRejected { get; set; }
}

public class ScanStatus
{
    public string State { get; set; } = "idle";
    public DateTime? StartedAt { get; set; }
    public DateTime? CompletedAt { get; set; }
    public int? RootFolderId { get; set; }
    public int FilesFound { get; set; }
    public int FilesImported { get; set; }
    public int FilesRejected { get; set; }
    public string? Error { get; set; }
}
