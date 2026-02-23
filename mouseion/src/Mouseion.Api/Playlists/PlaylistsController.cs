// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Playlists;

namespace Mouseion.Api.Playlists;

[ApiController]
[Route("api/v3/playlists")]
[Authorize]
public class PlaylistsController : ControllerBase
{
    private readonly IPlaylistRepository _playlistRepository;

    public PlaylistsController(IPlaylistRepository playlistRepository)
    {
        _playlistRepository = playlistRepository;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<PlaylistResource>>> GetPlaylists(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;

        var totalCount = await _playlistRepository.CountByUserAsync(userId, ct).ConfigureAwait(false);
        var playlists = await _playlistRepository.GetByUserAsync(userId, page, pageSize, ct).ConfigureAwait(false);

        var resources = new List<PlaylistResource>();
        foreach (var p in playlists)
        {
            var tracks = await _playlistRepository.GetTracksAsync(p.Id, ct).ConfigureAwait(false);
            resources.Add(ToResource(p, tracks.Count));
        }

        return Ok(new PagedResult<PlaylistResource>
        {
            Items = resources,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<PlaylistDetailResource>> GetPlaylist(int id, CancellationToken ct = default)
    {
        var playlist = await _playlistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (playlist == null) return NotFound();

        var tracks = await _playlistRepository.GetTracksAsync(id, ct).ConfigureAwait(false);
        return Ok(new PlaylistDetailResource
        {
            Id = playlist.Id,
            Name = playlist.Name,
            Description = playlist.Description,
            TrackCount = tracks.Count,
            Created = playlist.Created,
            Modified = playlist.Modified,
            TrackIds = tracks.Select(t => t.MediaItemId).ToList()
        });
    }

    [HttpPost]
    public async Task<ActionResult<PlaylistResource>> CreatePlaylist(
        [FromBody][Required] CreatePlaylistRequest request,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var playlist = await _playlistRepository.CreateAsync(new Playlist
        {
            UserId = userId,
            Name = request.Name,
            Description = request.Description
        }, ct).ConfigureAwait(false);

        return CreatedAtAction(nameof(GetPlaylist), new { id = playlist.Id }, ToResource(playlist, 0));
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<PlaylistResource>> UpdatePlaylist(
        int id,
        [FromBody][Required] UpdatePlaylistRequest request,
        CancellationToken ct = default)
    {
        var playlist = await _playlistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (playlist == null) return NotFound();

        playlist.Name = request.Name ?? playlist.Name;
        playlist.Description = request.Description ?? playlist.Description;

        var updated = await _playlistRepository.UpdateAsync(playlist, ct).ConfigureAwait(false);
        var tracks = await _playlistRepository.GetTracksAsync(id, ct).ConfigureAwait(false);
        return Ok(ToResource(updated, tracks.Count));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeletePlaylist(int id, CancellationToken ct = default)
    {
        var playlist = await _playlistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (playlist == null) return NotFound();

        await _playlistRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("{id:int}/tracks")]
    public async Task<IActionResult> AddTrack(int id, [FromBody][Required] AddTrackRequest request, CancellationToken ct = default)
    {
        var playlist = await _playlistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (playlist == null) return NotFound();

        await _playlistRepository.AddTrackAsync(id, request.MediaItemId, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpDelete("{id:int}/tracks/{mediaItemId:int}")]
    public async Task<IActionResult> RemoveTrack(int id, int mediaItemId, CancellationToken ct = default)
    {
        await _playlistRepository.RemoveTrackAsync(id, mediaItemId, ct).ConfigureAwait(false);
        return NoContent();
    }

    private int GetCurrentUserId()
    {
        var claim = User.FindFirst("userId")?.Value ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return int.TryParse(claim, out var id) ? id : 1;
    }

    private static PlaylistResource ToResource(Playlist p, int trackCount)
    {
        return new PlaylistResource
        {
            Id = p.Id,
            Name = p.Name,
            Description = p.Description,
            TrackCount = trackCount,
            Created = p.Created,
            Modified = p.Modified,
        };
    }
}

public class PlaylistResource
{
    public int Id { get; set; }
    public string Name { get; set; } = null!;
    public string? Description { get; set; }
    public int TrackCount { get; set; }
    public DateTime Created { get; set; }
    public DateTime Modified { get; set; }
}

public class PlaylistDetailResource : PlaylistResource
{
    public List<int> TrackIds { get; set; } = new();
}

public class CreatePlaylistRequest
{
    [Required]
    public string Name { get; set; } = null!;
    public string? Description { get; set; }
}

public class UpdatePlaylistRequest
{
    public string? Name { get; set; }
    public string? Description { get; set; }
}

public class AddTrackRequest
{
    [Required]
    public int MediaItemId { get; set; }
}
