// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Caching.Memory;
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

    public MusicScanController(IMusicFileScanner musicFileScanner, IMemoryCache cache)
    {
        _musicFileScanner = musicFileScanner;
        _cache = cache;
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
    public async Task<ActionResult<ScanResultResource>> ScanRootFolder(int id, CancellationToken ct = default)
    {
        var result = await _musicFileScanner.ScanRootFolderAsync(id, ct).ConfigureAwait(false);

        if (!result.Success)
        {
            return BadRequest(new { error = result.Error });
        }

        FacetsController.InvalidateCache(_cache);
        return Ok(ToResource(result));
    }

    [HttpPost("library")]
    public async Task<ActionResult<ScanResultResource>> ScanLibrary(CancellationToken ct = default)
    {
        var result = await _musicFileScanner.ScanLibraryAsync(ct).ConfigureAwait(false);

        if (!result.Success)
        {
            return BadRequest(new { error = result.Error });
        }

        FacetsController.InvalidateCache(_cache);
        return Ok(ToResource(result));
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
