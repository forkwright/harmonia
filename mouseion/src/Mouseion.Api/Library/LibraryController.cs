// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Filtering;
using Mouseion.Core.Library;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Music;

namespace Mouseion.Api.Library;

[ApiController]
[Route("api/v3/library")]
[Authorize]
public class LibraryController : ControllerBase
{
    private readonly ILibraryFilterService _filterService;

    public LibraryController(ILibraryFilterService filterService)
    {
        _filterService = filterService;
    }

    [HttpPost("filter")]
    public async Task<ActionResult<FilterPagedResult<TrackResource>>> FilterLibrary(
        [FromBody] FilterRequest request,
        CancellationToken ct = default)
    {
        var result = await _filterService.FilterTracksAsync(request, ct).ConfigureAwait(false);

        return Ok(new FilterPagedResult<TrackResource>
        {
            Items = result.Tracks.Select(ToResource).ToList(),
            Page = result.Page,
            PageSize = result.PageSize,
            TotalCount = result.TotalCount,
            Summary = result.Summary
        });
    }

    private static TrackResource ToResource(Track track)
    {
        return new TrackResource
        {
            Id = track.Id,
            AlbumId = track.AlbumId,
            ArtistId = track.ArtistId,
            Title = track.Title,
            ForeignTrackId = track.ForeignTrackId,
            MusicBrainzId = track.MusicBrainzId,
            TrackNumber = track.TrackNumber,
            DiscNumber = track.DiscNumber,
            DurationSeconds = track.DurationSeconds,
            Explicit = track.Explicit,
            MediaType = track.MediaType,
            Monitored = track.Monitored,
            QualityProfileId = track.QualityProfileId,
            Path = track.Path,
            RootFolderPath = track.RootFolderPath,
            Added = track.Added,
            Tags = track.Tags?.ToList(),
            LastSearchTime = track.LastSearchTime
        };
    }
}

public class TrackResource
{
    public int Id { get; set; }
    public int? AlbumId { get; set; }
    public int? ArtistId { get; set; }
    public string Title { get; set; } = null!;
    public string? ForeignTrackId { get; set; }
    public string? MusicBrainzId { get; set; }
    public int TrackNumber { get; set; }
    public int DiscNumber { get; set; }
    public int? DurationSeconds { get; set; }
    public bool Explicit { get; set; }
    public MediaType MediaType { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string Path { get; set; } = string.Empty;
    public string RootFolderPath { get; set; } = string.Empty;
    public DateTime Added { get; set; }
    public List<int>? Tags { get; set; }
    public DateTime? LastSearchTime { get; set; }
}
