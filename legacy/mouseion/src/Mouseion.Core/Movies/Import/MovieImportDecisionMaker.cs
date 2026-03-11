// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.Movies.Import.Specifications;

namespace Mouseion.Core.Movies.Import;

public class MovieImportDecisionMaker : IMovieImportDecisionMaker
{
    private readonly IEnumerable<IMovieImportSpecification> _specifications;
    private readonly ILogger<MovieImportDecisionMaker> _logger;

    public MovieImportDecisionMaker(
        IEnumerable<IMovieImportSpecification> specifications,
        ILogger<MovieImportDecisionMaker> logger)
    {
        _specifications = specifications;
        _logger = logger;
    }

    public async Task<List<MovieImportDecision>> GetImportDecisionsAsync(List<string> videoFiles, Movie movie, CancellationToken ct = default)
    {
        var decisions = new List<MovieImportDecision>();

        _logger.LogDebug("Analyzing {Count} video files for movie {Title} ({Year})", videoFiles.Count, movie.Title.SanitizeForLog(), movie.Year);

        foreach (var filePath in videoFiles)
        {
            var decision = await MakeDecisionAsync(filePath, movie, ct).ConfigureAwait(false);
            decisions.Add(decision);
        }

        _logger.LogInformation("Import decisions: {Approved} approved, {Rejected} rejected",
            decisions.Count(d => d.Approved),
            decisions.Count(d => !d.Approved));

        return decisions;
    }

    public List<MovieImportDecision> GetImportDecisions(List<string> videoFiles, Movie movie)
    {
        var decisions = new List<MovieImportDecision>();

        _logger.LogDebug("Analyzing {Count} video files for movie {Title} ({Year})", videoFiles.Count, movie.Title.SanitizeForLog(), movie.Year);

        foreach (var filePath in videoFiles)
        {
            var decision = MakeDecision(filePath, movie);
            decisions.Add(decision);
        }

        _logger.LogInformation("Import decisions: {Approved} approved, {Rejected} rejected",
            decisions.Count(d => d.Approved),
            decisions.Count(d => !d.Approved));

        return decisions;
    }

    private async Task<MovieImportDecision> MakeDecisionAsync(string filePath, Movie movie, CancellationToken ct)
    {
        var decision = new MovieImportDecision(filePath, movie);

        foreach (var specification in _specifications)
        {
            var rejection = await specification.IsSatisfiedByAsync(filePath, movie, ct).ConfigureAwait(false);
            if (rejection != null)
            {
                decision.AddRejection(rejection);
                _logger.LogDebug("File {FilePath} rejected: {Reason} - {Message}", filePath.SanitizeForLog(), rejection.Reason, rejection.Message.SanitizeForLog());
            }
        }

        if (decision.Approved)
        {
            _logger.LogDebug("File {FilePath} approved for import", filePath.SanitizeForLog());
        }

        return decision;
    }

    private MovieImportDecision MakeDecision(string filePath, Movie movie)
    {
        var decision = new MovieImportDecision(filePath, movie);

        foreach (var specification in _specifications)
        {
            var rejection = specification.IsSatisfiedBy(filePath, movie);
            if (rejection != null)
            {
                decision.AddRejection(rejection);
                _logger.LogDebug("File {FilePath} rejected: {Reason} - {Message}", filePath.SanitizeForLog(), rejection.Reason, rejection.Message.SanitizeForLog());
            }
        }

        if (decision.Approved)
        {
            _logger.LogDebug("File {FilePath} approved for import", filePath.SanitizeForLog());
        }

        return decision;
    }
}
