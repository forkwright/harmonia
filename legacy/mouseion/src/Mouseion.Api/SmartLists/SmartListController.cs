// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using Mouseion.Core.SmartLists;

namespace Mouseion.Api.SmartLists;

/// <summary>
/// API controller for Smart Lists — discovery-driven lists that auto-add items from external
/// metadata sources (TMDB, Trakt, AniList, MusicBrainz, OpenLibrary).
/// </summary>
[ApiController]
[Route("api/v3/smartlists")]
[Authorize]
public partial class SmartListController : ControllerBase
{
    private readonly ISmartListService _service;
    private readonly ILogger<SmartListController> _logger;

    public SmartListController(ISmartListService service, ILogger<SmartListController> logger)
    {
        _service = service;
        _logger = logger;
    }

    /// <summary>
    /// List all smart lists.
    /// </summary>
    [HttpGet]
    public async Task<ActionResult<List<SmartListResource>>> List(CancellationToken ct)
    {
        var lists = await _service.GetAllAsync(ct).ConfigureAwait(false);
        return Ok(lists.Select(l => l.ToResource()).ToList());
    }

    /// <summary>
    /// Get a specific smart list by ID.
    /// </summary>
    [HttpGet("{id:int}")]
    public async Task<ActionResult<SmartListResource>> Get(int id, CancellationToken ct)
    {
        var list = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (list == null) return NotFound();
        return Ok(list.ToResource());
    }

    /// <summary>
    /// Create a new smart list.
    /// </summary>
    [HttpPost]
    public async Task<ActionResult<SmartListResource>> Create(
        [FromBody][Required] SmartListResource resource, CancellationToken ct)
    {
        var entity = resource.ToModel();
        var created = await _service.CreateAsync(entity, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(Get), new { id = created.Id }, created.ToResource());
    }

    /// <summary>
    /// Update an existing smart list.
    /// </summary>
    [HttpPut("{id:int}")]
    public async Task<ActionResult<SmartListResource>> Update(
        int id, [FromBody][Required] SmartListResource resource, CancellationToken ct)
    {
        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null) return NotFound();

        var entity = resource.ToModel();
        entity.Id = id;
        entity.CreatedAt = existing.CreatedAt;
        entity.ItemsAdded = existing.ItemsAdded;
        entity.LastRefreshed = existing.LastRefreshed;
        var updated = await _service.UpdateAsync(entity, ct).ConfigureAwait(false);
        return Ok(updated.ToResource());
    }

    /// <summary>
    /// Delete a smart list and all its matches.
    /// </summary>
    [HttpDelete("{id:int}")]
    public async Task<ActionResult> Delete(int id, CancellationToken ct)
    {
        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null) return NotFound();

        await _service.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    /// <summary>
    /// Refresh a smart list — query the external source for new matches.
    /// </summary>
    [HttpPost("{id:int}/refresh")]
    public async Task<ActionResult<SmartListRefreshResult>> Refresh(int id, CancellationToken ct)
    {
        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null) return NotFound();

        var result = await _service.RefreshAsync(id, ct).ConfigureAwait(false);
        return Ok(result);
    }

    /// <summary>
    /// Refresh all smart lists that are due based on their configured intervals.
    /// </summary>
    [HttpPost("refresh")]
    public async Task<ActionResult<SmartListRefreshResult>> RefreshAllDue(CancellationToken ct)
    {
        var result = await _service.RefreshAllDueAsync(ct).ConfigureAwait(false);
        return Ok(result);
    }

    /// <summary>
    /// Get all discovered matches for a smart list.
    /// </summary>
    [HttpGet("{id:int}/matches")]
    public async Task<ActionResult<List<SmartListMatchResource>>> GetMatches(int id, CancellationToken ct)
    {
        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null) return NotFound();

        var matches = await _service.GetMatchesAsync(id, ct).ConfigureAwait(false);
        return Ok(matches.Select(m => m.ToResource()).ToList());
    }

    /// <summary>
    /// Skip/reject a discovered match — it won't be surfaced again.
    /// </summary>
    [HttpPost("matches/{matchId:int}/skip")]
    public async Task<ActionResult> SkipMatch(int matchId, CancellationToken ct)
    {
        try
        {
            await _service.SkipMatchAsync(matchId, ct).ConfigureAwait(false);
            return NoContent();
        }
        catch (KeyNotFoundException)
        {
            return NotFound();
        }
    }
}
