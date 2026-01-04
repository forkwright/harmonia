// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Movies;
using Mouseion.Core.Movies.Import;

namespace Mouseion.Api.Movies;

[ApiController]
[Route("api/v3/movies/import")]
[Authorize]
public class MovieImportController : ControllerBase
{
    private readonly IMovieRepository _movieRepository;
    private readonly IMovieImportDecisionMaker _decisionMaker;
    private readonly IImportApprovedMovies _importService;

    public MovieImportController(
        IMovieRepository movieRepository,
        IMovieImportDecisionMaker decisionMaker,
        IImportApprovedMovies importService)
    {
        _movieRepository = movieRepository;
        _decisionMaker = decisionMaker;
        _importService = importService;
    }

    /// <summary>
    /// Get import decisions for video files
    /// </summary>
    /// <param name="request">Import decision request</param>
    /// <param name="ct">Cancellation token</param>
    /// <returns>List of import decisions</returns>
    [HttpPost("decisions")]
    public async Task<ActionResult<List<ImportDecisionResource>>> GetImportDecisions(
        [FromBody] ImportDecisionRequest request,
        CancellationToken ct = default)
    {
        if (request.VideoFiles == null || !request.VideoFiles.Any())
        {
            return BadRequest(new { error = "No video files provided" });
        }

        var movie = await _movieRepository.FindAsync(request.MovieId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie {request.MovieId} not found" });
        }

        var decisions = await _decisionMaker.GetImportDecisionsAsync(request.VideoFiles, movie, ct).ConfigureAwait(false);

        return Ok(decisions.Select(ToResource).ToList());
    }

    /// <summary>
    /// Import approved video files
    /// </summary>
    /// <param name="request">Import request</param>
    /// <param name="ct">Cancellation token</param>
    /// <returns>Import results</returns>
    [HttpPost]
    public async Task<ActionResult<List<ImportResultResource>>> Import(
        [FromBody] ImportRequest request,
        CancellationToken ct = default)
    {
        if (request.VideoFiles == null || !request.VideoFiles.Any())
        {
            return BadRequest(new { error = "No video files provided" });
        }

        var movie = await _movieRepository.FindAsync(request.MovieId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            return NotFound(new { error = $"Movie {request.MovieId} not found" });
        }

        var decisions = await _decisionMaker.GetImportDecisionsAsync(request.VideoFiles, movie, ct).ConfigureAwait(false);
        var results = await _importService.ImportAsync(decisions, ct).ConfigureAwait(false);

        return Ok(results.Select(ToResultResource).ToList());
    }

    private static ImportDecisionResource ToResource(MovieImportDecision decision)
    {
        return new ImportDecisionResource
        {
            FilePath = decision.FilePath,
            MovieId = decision.Movie.Id,
            MovieTitle = decision.Movie.Title,
            Approved = decision.Approved,
            Rejections = decision.Rejections.Select(r => new RejectionResource
            {
                Reason = r.Reason.ToString(),
                Message = r.Message
            }).ToList()
        };
    }

    private static ImportResultResource ToResultResource(MovieImportResult result)
    {
        return new ImportResultResource
        {
            FilePath = result.Decision.FilePath,
            MovieId = result.Decision.Movie.Id,
            MovieTitle = result.Decision.Movie.Title,
            Success = result.Success,
            ErrorMessage = result.ErrorMessage
        };
    }
}

public class ImportDecisionRequest
{
    public int MovieId { get; set; }
    public List<string> VideoFiles { get; set; } = new();
}

public class ImportRequest
{
    public int MovieId { get; set; }
    public List<string> VideoFiles { get; set; } = new();
}

public class ImportDecisionResource
{
    public string FilePath { get; set; } = null!;
    public int MovieId { get; set; }
    public string MovieTitle { get; set; } = null!;
    public bool Approved { get; set; }
    public List<RejectionResource> Rejections { get; set; } = new();
}

public class RejectionResource
{
    public string Reason { get; set; } = null!;
    public string Message { get; set; } = null!;
}

public class ImportResultResource
{
    public string FilePath { get; set; } = null!;
    public int MovieId { get; set; }
    public string MovieTitle { get; set; } = null!;
    public bool Success { get; set; }
    public string? ErrorMessage { get; set; }
}
