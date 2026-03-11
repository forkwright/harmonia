// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;

namespace Mouseion.Api.Movies;

[ApiController]
[Route("api/v3/movies")]
[Authorize]
public class MovieController : ControllerBase
{
    private readonly IMovieRepository _movieRepository;
    private readonly IAddMovieService _addMovieService;

    public MovieController(
        IMovieRepository movieRepository,
        IAddMovieService addMovieService)
    {
        _movieRepository = movieRepository;
        _addMovieService = addMovieService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<MovieResource>>> GetMovies(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _movieRepository.CountAsync(ct).ConfigureAwait(false);
        var movies = await _movieRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<MovieResource>
        {
            Items = movies.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<MovieResource>> GetMovie(int id, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie {id} not found" });
        }

        return Ok(ToResource(movie));
    }

    [HttpGet("tmdb/{tmdbId}")]
    public async Task<ActionResult<MovieResource>> GetByTmdbId(string tmdbId, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindByTmdbIdAsync(tmdbId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie with TMDB ID {tmdbId} not found" });
        }

        return Ok(ToResource(movie));
    }

    [HttpGet("imdb/{imdbId}")]
    public async Task<ActionResult<MovieResource>> GetByImdbId(string imdbId, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindByImdbIdAsync(imdbId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie with IMDB ID {imdbId} not found" });
        }

        return Ok(ToResource(movie));
    }

    [HttpGet("collection/{collectionId:int}")]
    public async Task<ActionResult<List<MovieResource>>> GetMoviesByCollection(int collectionId, CancellationToken ct = default)
    {
        var movies = await _movieRepository.GetByCollectionIdAsync(collectionId, ct).ConfigureAwait(false);
        return Ok(movies.Select(ToResource).ToList());
    }

    [HttpPost]
    [ProducesResponseType(typeof(MovieResource), StatusCodes.Status201Created)]
    [ProducesResponseType(typeof(ApiProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ApiProblemDetails), StatusCodes.Status422UnprocessableEntity)]
    [ProducesResponseType(typeof(ApiProblemDetails), StatusCodes.Status500InternalServerError)]
    public async Task<ActionResult<MovieResource>> AddMovie([FromBody][Required] MovieResource resource, CancellationToken ct = default)
    {
        var movie = ToModel(resource);
        var added = await _addMovieService.AddMovieAsync(movie, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetMovie), new { id = added.Id }, ToResource(added));
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<MovieResource>>> AddMovies([FromBody][Required] List<MovieResource> resources, CancellationToken ct = default)
    {
        var movies = resources.Select(ToModel).ToList();
        var added = await _addMovieService.AddMoviesAsync(movies, ct).ConfigureAwait(false);
        return Ok(added.Select(ToResource).ToList());
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<MovieResource>> UpdateMovie(int id, [FromBody][Required] MovieResource resource, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie {id} not found" });
        }

        movie.Title = resource.Title;
        movie.Year = resource.Year;
        movie.Overview = resource.Overview;
        movie.Runtime = resource.Runtime;
        movie.TmdbId = resource.TmdbId;
        movie.ImdbId = resource.ImdbId;
        movie.Images = resource.Images ?? new List<string>();
        movie.Genres = resource.Genres ?? new List<string>();
        movie.InCinemas = resource.InCinemas;
        movie.PhysicalRelease = resource.PhysicalRelease;
        movie.DigitalRelease = resource.DigitalRelease;
        movie.Certification = resource.Certification;
        movie.Studio = resource.Studio;
        movie.Website = resource.Website;
        movie.YouTubeTrailerId = resource.YouTubeTrailerId;
        movie.Popularity = resource.Popularity;
        movie.CollectionId = resource.CollectionId;
        movie.Monitored = resource.Monitored;
        movie.QualityProfileId = resource.QualityProfileId;
        movie.Path = resource.Path;
        movie.RootFolderPath = resource.RootFolderPath;
        movie.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();

        var updated = await _movieRepository.UpdateAsync(movie, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteMovie(int id, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie {id} not found" });
        }

        await _movieRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static MovieResource ToResource(Movie movie)
    {
        return new MovieResource
        {
            Id = movie.Id,
            Title = movie.Title,
            Year = movie.Year,
            Overview = movie.Overview,
            Runtime = movie.Runtime,
            TmdbId = movie.TmdbId,
            ImdbId = movie.ImdbId,
            Images = movie.Images,
            Genres = movie.Genres,
            InCinemas = movie.InCinemas,
            PhysicalRelease = movie.PhysicalRelease,
            DigitalRelease = movie.DigitalRelease,
            Certification = movie.Certification,
            Studio = movie.Studio,
            Website = movie.Website,
            YouTubeTrailerId = movie.YouTubeTrailerId,
            Popularity = movie.Popularity,
            CollectionId = movie.CollectionId,
            MediaType = movie.MediaType,
            Monitored = movie.Monitored,
            QualityProfileId = movie.QualityProfileId,
            Path = movie.Path,
            RootFolderPath = movie.RootFolderPath,
            Added = movie.Added,
            Tags = movie.Tags?.ToList()
        };
    }

    private static Movie ToModel(MovieResource resource)
    {
        return new Movie
        {
            Id = resource.Id,
            Title = resource.Title,
            Year = resource.Year,
            Overview = resource.Overview,
            Runtime = resource.Runtime,
            TmdbId = resource.TmdbId,
            ImdbId = resource.ImdbId,
            Images = resource.Images ?? new List<string>(),
            Genres = resource.Genres ?? new List<string>(),
            InCinemas = resource.InCinemas,
            PhysicalRelease = resource.PhysicalRelease,
            DigitalRelease = resource.DigitalRelease,
            Certification = resource.Certification,
            Studio = resource.Studio,
            Website = resource.Website,
            YouTubeTrailerId = resource.YouTubeTrailerId,
            Popularity = resource.Popularity,
            CollectionId = resource.CollectionId,
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Path = resource.Path,
            RootFolderPath = resource.RootFolderPath,
            Added = resource.Added,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>()
        };
    }
}

public class MovieResource
{
    public int Id { get; set; }
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? Overview { get; set; }
    public int? Runtime { get; set; }
    public string? TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public List<string>? Images { get; set; }
    public List<string>? Genres { get; set; }
    public DateTime? InCinemas { get; set; }
    public DateTime? PhysicalRelease { get; set; }
    public DateTime? DigitalRelease { get; set; }
    public string? Certification { get; set; }
    public string? Studio { get; set; }
    public string? Website { get; set; }
    public string? YouTubeTrailerId { get; set; }
    public float? Popularity { get; set; }
    public int? CollectionId { get; set; }
    public MediaType MediaType { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string Path { get; set; } = string.Empty;
    public string RootFolderPath { get; set; } = string.Empty;
    public DateTime Added { get; set; }
    public List<int>? Tags { get; set; }
}
