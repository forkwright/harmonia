// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using Mouseion.Core.SmartPlaylists;

namespace Mouseion.Api.SmartPlaylists;

/// <summary>
/// API controller for managing smart playlists — dynamic playlists populated by filter criteria.
/// </summary>
[ApiController]
[Route("api/v3/smartplaylists")]
[Authorize]
public class SmartPlaylistController : ControllerBase
{
    private readonly ISmartPlaylistService _service;
    private readonly ILogger<SmartPlaylistController> _logger;

    public SmartPlaylistController(
        ISmartPlaylistService service,
        ILogger<SmartPlaylistController> logger)
    {
        _service = service;
        _logger = logger;
    }

    /// <summary>
    /// List all smart playlists.
    /// </summary>
    [HttpGet]
    public async Task<ActionResult<List<SmartPlaylistResource>>> List(CancellationToken ct)
    {
        _logger.LogDebug("Listing all smart playlists");

        var playlists = await _service.GetAllAsync(ct).ConfigureAwait(false);
        var resources = playlists.Select(p => ToResource(p)).ToList();

        return Ok(resources);
    }

    /// <summary>
    /// Get a specific smart playlist by ID, including its tracks.
    /// </summary>
    [HttpGet("{id:int}")]
    public async Task<ActionResult<SmartPlaylistResource>> Get(int id, CancellationToken ct)
    {
        _logger.LogDebug("Getting smart playlist {Id}", id);

        var playlist = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (playlist == null)
        {
            return NotFound();
        }

        var tracks = await _service.GetTracksAsync(id, ct).ConfigureAwait(false);
        return Ok(ToResource(playlist, tracks));
    }

    /// <summary>
    /// Create a new smart playlist.
    /// </summary>
    [HttpPost]
    public async Task<ActionResult<SmartPlaylistResource>> Create(
        [FromBody][Required] SmartPlaylistResource resource,
        CancellationToken ct)
    {
        _logger.LogDebug("Creating smart playlist: {Name}", resource.Name);

        var entity = ToEntity(resource);
        var created = await _service.CreateAsync(entity, ct).ConfigureAwait(false);

        return CreatedAtAction(nameof(Get), new { id = created.Id }, ToResource(created));
    }

    /// <summary>
    /// Update an existing smart playlist.
    /// </summary>
    [HttpPut("{id:int}")]
    public async Task<ActionResult<SmartPlaylistResource>> Update(
        int id,
        [FromBody][Required] SmartPlaylistResource resource,
        CancellationToken ct)
    {
        _logger.LogDebug("Updating smart playlist {Id}", id);

        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null)
        {
            return NotFound();
        }

        var entity = ToEntity(resource);
        entity.Id = id;
        entity.CreatedAt = existing.CreatedAt;
        var updated = await _service.UpdateAsync(entity, ct).ConfigureAwait(false);

        return Ok(ToResource(updated));
    }

    /// <summary>
    /// Delete a smart playlist.
    /// </summary>
    [HttpDelete("{id:int}")]
    public async Task<ActionResult> Delete(int id, CancellationToken ct)
    {
        _logger.LogDebug("Deleting smart playlist {Id}", id);

        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null)
        {
            return NotFound();
        }

        await _service.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    /// <summary>
    /// Refresh a smart playlist — re-evaluate its filter and repopulate tracks.
    /// </summary>
    [HttpPost("{id:int}/refresh")]
    public async Task<ActionResult<SmartPlaylistResource>> Refresh(int id, CancellationToken ct)
    {
        _logger.LogDebug("Refreshing smart playlist {Id}", id);

        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null)
        {
            return NotFound();
        }

        var refreshed = await _service.RefreshAsync(id, ct).ConfigureAwait(false);
        var tracks = await _service.GetTracksAsync(id, ct).ConfigureAwait(false);

        return Ok(ToResource(refreshed, tracks));
    }

    private static SmartPlaylistResource ToResource(
        SmartPlaylist playlist,
        IEnumerable<SmartPlaylistTrack>? tracks = null)
    {
        return new SmartPlaylistResource
        {
            Id = playlist.Id,
            Name = playlist.Name,
            FilterRequestJson = playlist.FilterRequestJson,
            TrackCount = playlist.TrackCount,
            LastRefreshed = playlist.LastRefreshed,
            CreatedAt = playlist.CreatedAt,
            UpdatedAt = playlist.UpdatedAt,
            Tracks = tracks?.Select(t => new SmartPlaylistTrackResource
            {
                TrackId = t.TrackId,
                Position = t.Position
            }).ToList()
        };
    }

    private static SmartPlaylist ToEntity(SmartPlaylistResource resource)
    {
        return new SmartPlaylist
        {
            Id = resource.Id,
            Name = resource.Name,
            FilterRequestJson = resource.FilterRequestJson,
            TrackCount = resource.TrackCount,
            LastRefreshed = resource.LastRefreshed,
            CreatedAt = resource.CreatedAt,
            UpdatedAt = resource.UpdatedAt
        };
    }
}
