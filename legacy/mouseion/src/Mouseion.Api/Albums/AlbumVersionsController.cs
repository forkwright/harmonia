// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Music;

namespace Mouseion.Api.Albums;

[ApiController]
[Route("api/v3/albums/{id:int}/versions")]
[Authorize]
public class AlbumVersionsController : ControllerBase
{
    private readonly IAlbumVersionsService _versionsService;

    public AlbumVersionsController(IAlbumVersionsService versionsService)
    {
        _versionsService = versionsService;
    }

    [HttpGet]
    public async Task<ActionResult<AlbumVersionsResource>> GetVersions(int id, CancellationToken ct = default)
    {
        var result = await _versionsService.GetVersionsAsync(id, ct).ConfigureAwait(false);
        if (result == null)
        {
            return NotFound(new { error = $"Album {id} not found" });
        }

        return Ok(new AlbumVersionsResource
        {
            Canonical = ToResource(result.Canonical),
            Versions = result.Versions.Select(ToResource).ToList()
        });
    }

    private static AlbumResource ToResource(Album album)
    {
        return new AlbumResource
        {
            Id = album.Id,
            ArtistId = album.ArtistId,
            Title = album.Title,
            SortTitle = album.SortTitle,
            Description = album.Description,
            ForeignAlbumId = album.ForeignAlbumId,
            DiscogsId = album.DiscogsId,
            MusicBrainzId = album.MusicBrainzId,
            ReleaseGroupMbid = album.ReleaseGroupMbid,
            ReleaseStatus = album.ReleaseStatus,
            ReleaseCountry = album.ReleaseCountry,
            RecordLabel = album.RecordLabel,
            MediaType = album.MediaType,
            ReleaseDate = album.ReleaseDate,
            AlbumType = album.AlbumType,
            Images = album.Images ?? new List<string>(),
            Rating = album.Rating,
            Votes = album.Votes,
            Genres = album.Genres ?? new List<string>(),
            TrackCount = album.TrackCount,
            DiscCount = album.DiscCount,
            Duration = album.Duration,
            Monitored = album.Monitored,
            QualityProfileId = album.QualityProfileId,
            Path = album.Path,
            RootFolderPath = album.RootFolderPath,
            Added = album.Added,
            Tags = album.Tags?.ToList(),
            LastSearchTime = album.LastSearchTime
        };
    }
}
