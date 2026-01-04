// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Movies;

namespace Mouseion.Core.Movies.Import;

public class ImportApprovedMovies : IImportApprovedMovies
{
    private readonly IMovieFileRepository _movieFileRepository;
    private readonly ILogger<ImportApprovedMovies> _logger;

    public ImportApprovedMovies(
        IMovieFileRepository movieFileRepository,
        ILogger<ImportApprovedMovies> logger)
    {
        _movieFileRepository = movieFileRepository;
        _logger = logger;
    }

    public async Task<List<MovieImportResult>> ImportAsync(List<MovieImportDecision> decisions, CancellationToken ct = default)
    {
        var results = new List<MovieImportResult>();

        var approvedDecisions = decisions.Where(d => d.Approved).ToList();
        _logger.LogInformation("Importing {Count} approved movie files", approvedDecisions.Count);

        foreach (var decision in approvedDecisions)
        {
            try
            {
                var movieFile = new MovieFile
                {
                    MovieId = decision.Movie.Id,
                    Path = decision.FilePath,
                    DateAdded = DateTime.UtcNow
                };

                await _movieFileRepository.InsertAsync(movieFile, ct).ConfigureAwait(false);

                _logger.LogInformation("Imported movie file: {FilePath} for {Title} ({Year})",
                    decision.FilePath, decision.Movie.Title, decision.Movie.Year);

                results.Add(new MovieImportResult(decision, success: true));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to import movie file: {FilePath}", decision.FilePath);
                results.Add(new MovieImportResult(decision, success: false, errorMessage: ex.Message));
            }
        }

        var rejectedDecisions = decisions.Where(d => !d.Approved).ToList();
        foreach (var decision in rejectedDecisions)
        {
            var reasons = string.Join(", ", decision.Rejections.Select(r => r.Message));
            results.Add(new MovieImportResult(decision, success: false, errorMessage: $"Rejected: {reasons}"));
        }

        return results;
    }

    public List<MovieImportResult> Import(List<MovieImportDecision> decisions)
    {
        var results = new List<MovieImportResult>();

        var approvedDecisions = decisions.Where(d => d.Approved).ToList();
        _logger.LogInformation("Importing {Count} approved movie files", approvedDecisions.Count);

        foreach (var decision in approvedDecisions)
        {
            try
            {
                var movieFile = new MovieFile
                {
                    MovieId = decision.Movie.Id,
                    Path = decision.FilePath,
                    DateAdded = DateTime.UtcNow
                };

                _movieFileRepository.Insert(movieFile);

                _logger.LogInformation("Imported movie file: {FilePath} for {Title} ({Year})",
                    decision.FilePath, decision.Movie.Title, decision.Movie.Year);

                results.Add(new MovieImportResult(decision, success: true));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to import movie file: {FilePath}", decision.FilePath);
                results.Add(new MovieImportResult(decision, success: false, errorMessage: ex.Message));
            }
        }

        var rejectedDecisions = decisions.Where(d => !d.Approved).ToList();
        foreach (var decision in rejectedDecisions)
        {
            var reasons = string.Join(", ", decision.Rejections.Select(r => r.Message));
            results.Add(new MovieImportResult(decision, success: false, errorMessage: $"Rejected: {reasons}"));
        }

        return results;
    }
}
