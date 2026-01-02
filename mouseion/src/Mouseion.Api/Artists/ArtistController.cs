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
using Mouseion.Core.Music;

namespace Mouseion.Api.Artists;

[ApiController]
[Route("api/v3/artists/music")]
[Authorize]
public class ArtistController : ControllerBase
{
    private readonly IArtistRepository _artistRepository;
    private readonly IAddArtistService _addArtistService;
    private readonly IArtistStatisticsService _statisticsService;

    public ArtistController(
        IArtistRepository artistRepository,
        IAddArtistService addArtistService,
        IArtistStatisticsService statisticsService)
    {
        _artistRepository = artistRepository;
        _addArtistService = addArtistService;
        _statisticsService = statisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<ArtistResource>>> GetArtists(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _artistRepository.CountAsync(ct).ConfigureAwait(false);
        var artists = await _artistRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<ArtistResource>
        {
            Items = artists.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<ArtistResource>> GetArtist(int id, CancellationToken ct = default)
    {
        var artist = await _artistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (artist == null)
        {
            return NotFound(new { error = $"Artist {id} not found" });
        }

        return Ok(ToResource(artist));
    }

    [HttpGet("musicbrainz/{musicBrainzId}")]
    public async Task<ActionResult<ArtistResource>> GetByMusicBrainzId(string musicBrainzId, CancellationToken ct = default)
    {
        var artist = await _artistRepository.FindByMusicBrainzIdAsync(musicBrainzId, ct).ConfigureAwait(false);
        if (artist == null)
        {
            return NotFound(new { error = $"Artist with MusicBrainz ID {musicBrainzId} not found" });
        }

        return Ok(ToResource(artist));
    }

    [HttpGet("name/{name}")]
    public async Task<ActionResult<ArtistResource>> GetByName(string name, CancellationToken ct = default)
    {
        var artist = await _artistRepository.FindByNameAsync(name, ct).ConfigureAwait(false);
        if (artist == null)
        {
            return NotFound(new { error = $"Artist '{name}' not found" });
        }

        return Ok(ToResource(artist));
    }

    [HttpGet("statistics/{artistId:int}")]
    public async Task<ActionResult<ArtistStatistics>> GetStatistics(int artistId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(artistId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpPost]
    public async Task<ActionResult<ArtistResource>> AddArtist([FromBody] ArtistResource resource, CancellationToken ct = default)
    {
        try
        {
            var artist = ToModel(resource);
            var added = await _addArtistService.AddArtistAsync(artist, ct).ConfigureAwait(false);
            return CreatedAtAction(nameof(GetArtist), new { id = added.Id }, ToResource(added));
        }
        catch (ArgumentException ex)
        {
            return BadRequest(new { error = ex.Message });
        }
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<ArtistResource>>> AddArtists([FromBody] List<ArtistResource> resources, CancellationToken ct = default)
    {
        try
        {
            var artists = resources.Select(ToModel).ToList();
            var added = await _addArtistService.AddArtistsAsync(artists, ct).ConfigureAwait(false);
            return Ok(added.Select(ToResource).ToList());
        }
        catch (ArgumentException ex)
        {
            return BadRequest(new { error = ex.Message });
        }
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<ArtistResource>> UpdateArtist(int id, [FromBody] ArtistResource resource, CancellationToken ct = default)
    {
        var artist = await _artistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (artist == null)
        {
            return NotFound(new { error = $"Artist {id} not found" });
        }

        artist.Name = resource.Name;
        artist.SortName = resource.SortName;
        artist.Description = resource.Description;
        artist.MusicBrainzId = resource.MusicBrainzId;
        artist.SpotifyId = resource.SpotifyId;
        artist.LastFmId = resource.LastFmId;
        artist.Images = resource.Images ?? new List<string>();
        artist.Rating = resource.Rating;
        artist.Votes = resource.Votes;
        artist.Genres = resource.Genres ?? new List<string>();
        artist.Country = resource.Country;
        artist.BeginDate = resource.BeginDate;
        artist.EndDate = resource.EndDate;
        artist.Monitored = resource.Monitored;
        artist.QualityProfileId = resource.QualityProfileId;
        artist.Path = resource.Path;
        artist.RootFolderPath = resource.RootFolderPath;
        artist.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();

        var updated = await _artistRepository.UpdateAsync(artist, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteArtist(int id, CancellationToken ct = default)
    {
        var artist = await _artistRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (artist == null)
        {
            return NotFound(new { error = $"Artist {id} not found" });
        }

        await _artistRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static ArtistResource ToResource(Artist artist)
    {
        return new ArtistResource
        {
            Id = artist.Id,
            Name = artist.Name,
            SortName = artist.SortName,
            Description = artist.Description,
            MusicBrainzId = artist.MusicBrainzId,
            SpotifyId = artist.SpotifyId,
            LastFmId = artist.LastFmId,
            Images = artist.Images ?? new List<string>(),
            Rating = artist.Rating,
            Votes = artist.Votes,
            Genres = artist.Genres ?? new List<string>(),
            Country = artist.Country,
            BeginDate = artist.BeginDate,
            EndDate = artist.EndDate,
            Monitored = artist.Monitored,
            QualityProfileId = artist.QualityProfileId,
            Path = artist.Path,
            RootFolderPath = artist.RootFolderPath,
            Added = artist.Added,
            Tags = artist.Tags?.ToList()
        };
    }

    private static Artist ToModel(ArtistResource resource)
    {
        return new Artist
        {
            Id = resource.Id,
            Name = resource.Name,
            SortName = resource.SortName,
            Description = resource.Description,
            MusicBrainzId = resource.MusicBrainzId,
            SpotifyId = resource.SpotifyId,
            LastFmId = resource.LastFmId,
            Images = resource.Images ?? new List<string>(),
            Rating = resource.Rating,
            Votes = resource.Votes,
            Genres = resource.Genres ?? new List<string>(),
            Country = resource.Country,
            BeginDate = resource.BeginDate,
            EndDate = resource.EndDate,
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Path = resource.Path,
            RootFolderPath = resource.RootFolderPath,
            Added = resource.Added,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>()
        };
    }
}

public class ArtistResource
{
    public int Id { get; set; }
    public string Name { get; set; } = null!;
    public string? SortName { get; set; }
    public string? Description { get; set; }
    public string? MusicBrainzId { get; set; }
    public string? SpotifyId { get; set; }
    public string? LastFmId { get; set; }
    public List<string>? Images { get; set; }
    public decimal? Rating { get; set; }
    public int? Votes { get; set; }
    public List<string>? Genres { get; set; }
    public string? Country { get; set; }
    public DateTime? BeginDate { get; set; }
    public DateTime? EndDate { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public DateTime Added { get; set; }
    public List<int>? Tags { get; set; }
}
