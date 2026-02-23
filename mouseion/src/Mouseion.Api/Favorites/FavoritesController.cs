// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Api.Resources;
using Mouseion.Core.Favorites;
using Mouseion.Core.Music;

namespace Mouseion.Api.Favorites;

[ApiController]
[Route("api/v3/favorites")]
[Authorize]
public class FavoritesController : ControllerBase
{
    private readonly IFavoriteRepository _favoriteRepository;
    private readonly ITrackRepository _trackRepository;

    public FavoritesController(
        IFavoriteRepository favoriteRepository,
        ITrackRepository trackRepository)
    {
        _favoriteRepository = favoriteRepository;
        _trackRepository = trackRepository;
    }

    /// <summary>Get all favorite media item IDs for the current user.</summary>
    [HttpGet("ids")]
    public async Task<ActionResult<List<int>>> GetFavoriteIds(CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        var ids = await _favoriteRepository.GetFavoriteIdsAsync(userId, ct).ConfigureAwait(false);
        return Ok(ids);
    }

    /// <summary>Get favorite tracks with full details (paged).</summary>
    [HttpGet]
    public async Task<ActionResult<PagedResult<TrackResource>>> GetFavorites(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _favoriteRepository.CountAsync(userId, ct).ConfigureAwait(false);
        var favorites = await _favoriteRepository.GetFavoritesPagedAsync(userId, page, pageSize, ct).ConfigureAwait(false);

        var trackIds = favorites.Select(f => f.MediaItemId).ToList();
        var tracks = await _trackRepository.GetByIdsAsync(trackIds, ct).ConfigureAwait(false);

        var resources = tracks.Select(t => new TrackResource
        {
            Id = t.Id,
            Title = t.Title,
            ForeignTrackId = t.ForeignTrackId,
            MusicBrainzId = t.MusicBrainzId,
            TrackNumber = t.TrackNumber,
            DiscNumber = t.DiscNumber,
            DurationSeconds = t.DurationSeconds,
            Explicit = t.Explicit,
            ArtistName = t.ArtistName,
            AlbumName = t.AlbumName,
        }).ToList();

        return Ok(new PagedResult<TrackResource>
        {
            Items = resources,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    /// <summary>Add a media item to favorites.</summary>
    [HttpPost("{mediaItemId:int}")]
    public async Task<IActionResult> AddFavorite(int mediaItemId, CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        await _favoriteRepository.AddAsync(userId, mediaItemId, ct).ConfigureAwait(false);
        return NoContent();
    }

    /// <summary>Remove a media item from favorites.</summary>
    [HttpDelete("{mediaItemId:int}")]
    public async Task<IActionResult> RemoveFavorite(int mediaItemId, CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        await _favoriteRepository.RemoveAsync(userId, mediaItemId, ct).ConfigureAwait(false);
        return NoContent();
    }

    private int GetCurrentUserId()
    {
        var claim = User.FindFirst("userId")?.Value ?? User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return int.TryParse(claim, out var id) ? id : 1;
    }
}
