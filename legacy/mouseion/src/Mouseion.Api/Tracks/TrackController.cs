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
using Mouseion.Api.Common;
using Mouseion.Api.Resources;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Music;

namespace Mouseion.Api.Tracks;

[ApiController]
[Route("api/v3/tracks")]
[Authorize]
public class TrackController : ControllerBase
{
    private readonly ITrackRepository _trackRepository;
    private readonly IAddTrackService _addTrackService;
    private readonly IMusicFileRepository _musicFileRepository;

    public TrackController(
        ITrackRepository trackRepository,
        IAddTrackService addTrackService,
        IMusicFileRepository musicFileRepository)
    {
        _trackRepository = trackRepository;
        _addTrackService = addTrackService;
        _musicFileRepository = musicFileRepository;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<TrackResource>>> GetTracks(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _trackRepository.CountAsync(ct).ConfigureAwait(false);
        var tracks = await _trackRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        var enriched = await ToResourcesWithFilesAsync(tracks, ct).ConfigureAwait(false);
        return Ok(new PagedResult<TrackResource>
        {
            Items = enriched,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<TrackResource>> GetTrack(int id, CancellationToken ct = default)
    {
        var track = await _trackRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (track == null)
        {
            return NotFound(new { error = $"Track {id} not found" });
        }

        return Ok(await ToResourceWithFileAsync(track, ct).ConfigureAwait(false));
    }

    [HttpGet("album/{albumId:int}")]
    public async Task<ActionResult<List<TrackResource>>> GetTracksByAlbum(int albumId, CancellationToken ct = default)
    {
        var tracks = await _trackRepository.GetByAlbumIdAsync(albumId, ct).ConfigureAwait(false);
        return Ok(await ToResourcesWithFilesAsync(tracks, ct).ConfigureAwait(false));
    }

    [HttpGet("artist/{artistId:int}")]
    public async Task<ActionResult<List<TrackResource>>> GetTracksByArtist(int artistId, CancellationToken ct = default)
    {
        var tracks = await _trackRepository.GetByArtistIdAsync(artistId, ct).ConfigureAwait(false);
        return Ok(await ToResourcesWithFilesAsync(tracks, ct).ConfigureAwait(false));
    }

    [HttpGet("foreignId/{foreignTrackId}")]
    public async Task<ActionResult<TrackResource>> GetByForeignId(string foreignTrackId, CancellationToken ct = default)
    {
        var track = await _trackRepository.FindByForeignIdAsync(foreignTrackId, ct).ConfigureAwait(false);
        if (track == null)
        {
            return NotFound(new { error = $"Track with MusicBrainz ID {foreignTrackId} not found" });
        }

        return Ok(ToResource(track));
    }

    [HttpPost]
    public async Task<ActionResult<TrackResource>> AddTrack([FromBody][Required] TrackResource resource, CancellationToken ct = default)
    {
        var track = ToModel(resource);
        var added = await _addTrackService.AddTrackAsync(track, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetTrack), new { id = added.Id }, ToResource(added));
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<TrackResource>>> AddTracks([FromBody][Required] List<TrackResource> resources, CancellationToken ct = default)
    {
        var tracks = resources.Select(ToModel).ToList();
        var added = await _addTrackService.AddTracksAsync(tracks, ct).ConfigureAwait(false);
        return Ok(added.Select(t => ToResource(t)).ToList());
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<TrackResource>> UpdateTrack(int id, [FromBody][Required] TrackResource resource, CancellationToken ct = default)
    {
        var track = await _trackRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (track == null)
        {
            return NotFound(new { error = $"Track {id} not found" });
        }

        track.AlbumId = resource.AlbumId;
        track.ArtistId = resource.ArtistId;
        track.Title = resource.Title;
        track.ForeignTrackId = resource.ForeignTrackId;
        track.MusicBrainzId = resource.MusicBrainzId;
        track.TrackNumber = resource.TrackNumber;
        track.DiscNumber = resource.DiscNumber;
        track.DurationSeconds = resource.DurationSeconds;
        track.Explicit = resource.Explicit;
        track.Monitored = resource.Monitored;
        track.QualityProfileId = resource.QualityProfileId;
        track.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();

        var updated = await _trackRepository.UpdateAsync(track, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteTrack(int id, CancellationToken ct = default)
    {
        var track = await _trackRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (track == null)
        {
            return NotFound(new { error = $"Track {id} not found" });
        }

        await _trackRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static TrackResource ToResource(Track track, MusicFile? musicFile = null)
        => TrackResourceMapper.ToResource(track, musicFile);

    private Task<TrackResource> ToResourceWithFileAsync(Track track, CancellationToken ct)
        => TrackResourceMapper.ToResourceWithFileAsync(track, _musicFileRepository, ct);

    private Task<List<TrackResource>> ToResourcesWithFilesAsync(IEnumerable<Track> tracks, CancellationToken ct)
        => TrackResourceMapper.ToResourcesWithFilesAsync(tracks, _musicFileRepository, ct);

    private static Track ToModel(TrackResource resource)
    {
        return new Track
        {
            Id = resource.Id,
            AlbumId = resource.AlbumId,
            ArtistId = resource.ArtistId,
            Title = resource.Title,
            ForeignTrackId = resource.ForeignTrackId,
            MusicBrainzId = resource.MusicBrainzId,
            TrackNumber = resource.TrackNumber,
            DiscNumber = resource.DiscNumber,
            DurationSeconds = resource.DurationSeconds,
            Explicit = resource.Explicit,
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Added = resource.Added,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>()
        };
    }
}
