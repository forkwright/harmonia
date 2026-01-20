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
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Music;

namespace Mouseion.Api.Albums;

[ApiController]
[Route("api/v3/albums")]
[Authorize]
public class AlbumController : ControllerBase
{
    private readonly IAlbumRepository _albumRepository;
    private readonly IAddAlbumService _addAlbumService;

    public AlbumController(
        IAlbumRepository albumRepository,
        IAddAlbumService addAlbumService)
    {
        _albumRepository = albumRepository;
        _addAlbumService = addAlbumService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<AlbumResource>>> GetAlbums(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _albumRepository.CountAsync(ct).ConfigureAwait(false);
        var albums = await _albumRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<AlbumResource>
        {
            Items = albums.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<AlbumResource>> GetAlbum(int id, CancellationToken ct = default)
    {
        var album = await _albumRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (album == null)
        {
            return NotFound(new { error = $"Album {id} not found" });
        }

        return Ok(ToResource(album));
    }

    [HttpGet("artist/{artistId:int}")]
    public async Task<ActionResult<List<AlbumResource>>> GetAlbumsByArtist(int artistId, CancellationToken ct = default)
    {
        var albums = await _albumRepository.GetByArtistIdAsync(artistId, ct).ConfigureAwait(false);
        return Ok(albums.Select(ToResource).ToList());
    }

    [HttpGet("musicbrainz/{foreignAlbumId}")]
    public async Task<ActionResult<AlbumResource>> GetByForeignId(string foreignAlbumId, CancellationToken ct = default)
    {
        var album = await _albumRepository.FindByForeignIdAsync(foreignAlbumId, ct).ConfigureAwait(false);
        if (album == null)
        {
            return NotFound(new { error = $"Album with MusicBrainz ID {foreignAlbumId} not found" });
        }

        return Ok(ToResource(album));
    }

    [HttpPost]
    public async Task<ActionResult<AlbumResource>> AddAlbum([FromBody][Required] AlbumResource resource, CancellationToken ct = default)
    {
        var album = ToModel(resource);
        var added = await _addAlbumService.AddAlbumAsync(album, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetAlbum), new { id = added.Id }, ToResource(added));
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<AlbumResource>>> AddAlbums([FromBody][Required] List<AlbumResource> resources, CancellationToken ct = default)
    {
        var albums = resources.Select(ToModel).ToList();
        var added = await _addAlbumService.AddAlbumsAsync(albums, ct).ConfigureAwait(false);
        return Ok(added.Select(ToResource).ToList());
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<AlbumResource>> UpdateAlbum(int id, [FromBody][Required] AlbumResource resource, CancellationToken ct = default)
    {
        var album = await _albumRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (album == null)
        {
            return NotFound(new { error = $"Album {id} not found" });
        }

        album.ArtistId = resource.ArtistId;
        album.Title = resource.Title;
        album.SortTitle = resource.SortTitle;
        album.Description = resource.Description;
        album.ForeignAlbumId = resource.ForeignAlbumId;
        album.DiscogsId = resource.DiscogsId;
        album.MusicBrainzId = resource.MusicBrainzId;
        album.ReleaseGroupMbid = resource.ReleaseGroupMbid;
        album.ReleaseStatus = resource.ReleaseStatus;
        album.ReleaseCountry = resource.ReleaseCountry;
        album.RecordLabel = resource.RecordLabel;
        album.ReleaseDate = resource.ReleaseDate;
        album.AlbumType = resource.AlbumType;
        album.Images = resource.Images ?? new List<string>();
        album.Rating = resource.Rating;
        album.Votes = resource.Votes;
        album.Genres = resource.Genres ?? new List<string>();
        album.TrackCount = resource.TrackCount;
        album.DiscCount = resource.DiscCount;
        album.Duration = resource.Duration;
        album.Monitored = resource.Monitored;
        album.QualityProfileId = resource.QualityProfileId;
        album.Path = resource.Path;
        album.RootFolderPath = resource.RootFolderPath;
        album.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();

        var updated = await _albumRepository.UpdateAsync(album, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteAlbum(int id, CancellationToken ct = default)
    {
        var album = await _albumRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (album == null)
        {
            return NotFound(new { error = $"Album {id} not found" });
        }

        await _albumRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
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

    private static Album ToModel(AlbumResource resource)
    {
        return new Album
        {
            Id = resource.Id,
            ArtistId = resource.ArtistId,
            Title = resource.Title,
            SortTitle = resource.SortTitle,
            Description = resource.Description,
            ForeignAlbumId = resource.ForeignAlbumId,
            DiscogsId = resource.DiscogsId,
            MusicBrainzId = resource.MusicBrainzId,
            ReleaseGroupMbid = resource.ReleaseGroupMbid,
            ReleaseStatus = resource.ReleaseStatus,
            ReleaseCountry = resource.ReleaseCountry,
            RecordLabel = resource.RecordLabel,
            MediaType = resource.MediaType,
            ReleaseDate = resource.ReleaseDate,
            AlbumType = resource.AlbumType,
            Images = resource.Images ?? new List<string>(),
            Rating = resource.Rating,
            Votes = resource.Votes,
            Genres = resource.Genres ?? new List<string>(),
            TrackCount = resource.TrackCount,
            DiscCount = resource.DiscCount,
            Duration = resource.Duration,
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Path = resource.Path,
            RootFolderPath = resource.RootFolderPath,
            Added = resource.Added,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>(),
            LastSearchTime = resource.LastSearchTime
        };
    }
}

public class AlbumResource
{
    public int Id { get; set; }
    public int? ArtistId { get; set; }
    public string Title { get; set; } = null!;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? ForeignAlbumId { get; set; }
    public string? DiscogsId { get; set; }
    public string? MusicBrainzId { get; set; }
    public string? ReleaseGroupMbid { get; set; }
    public string? ReleaseStatus { get; set; }
    public string? ReleaseCountry { get; set; }
    public string? RecordLabel { get; set; }
    public MediaType MediaType { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? AlbumType { get; set; }
    public List<string>? Images { get; set; }
    public decimal? Rating { get; set; }
    public int? Votes { get; set; }
    public List<string>? Genres { get; set; }
    public int? TrackCount { get; set; }
    public int? DiscCount { get; set; }
    public int? Duration { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public DateTime Added { get; set; }
    public List<int>? Tags { get; set; }
    public DateTime? LastSearchTime { get; set; }
}

public class AlbumVersionsResource
{
    public AlbumResource Canonical { get; set; } = null!;
    public List<AlbumResource> Versions { get; set; } = new();
}
