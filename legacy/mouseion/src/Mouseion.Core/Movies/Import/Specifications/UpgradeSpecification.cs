// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.Parser;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Movies.Import.Specifications;

public class UpgradeSpecification : IMovieImportSpecification
{
    private readonly IMovieFileRepository _movieFileRepository;
    private readonly ILogger<UpgradeSpecification> _logger;

    public UpgradeSpecification(
        IMovieFileRepository movieFileRepository,
        ILogger<UpgradeSpecification> logger)
    {
        _movieFileRepository = movieFileRepository;
        _logger = logger;
    }

    public async Task<ImportRejection?> IsSatisfiedByAsync(string filePath, Movie movie, CancellationToken ct = default)
    {
        // Parse quality from candidate file
        var candidateQuality = QualityParser.ParseQuality(filePath, _logger);

        if (candidateQuality.Quality == Quality.Unknown)
        {
            _logger.LogDebug("Cannot determine quality for {FilePath}, allowing import", filePath.SanitizeForLog());
            return null; // Allow import if quality cannot be determined
        }

        // Get existing movie file (if any)
        var existingFile = await _movieFileRepository.FindByMovieIdAsync(movie.Id, ct).ConfigureAwait(false);

        return CheckUpgrade(candidateQuality, existingFile, movie, filePath);
    }

    public ImportRejection? IsSatisfiedBy(string filePath, Movie movie)
    {
        // Parse quality from candidate file
        var candidateQuality = QualityParser.ParseQuality(filePath, _logger);

        if (candidateQuality.Quality == Quality.Unknown)
        {
            _logger.LogDebug("Cannot determine quality for {FilePath}, allowing import", filePath.SanitizeForLog());
            return null;
        }

        // Get existing movie file (if any)
        var existingFile = _movieFileRepository.FindByMovieId(movie.Id);

        return CheckUpgrade(candidateQuality, existingFile, movie, filePath);
    }

    private ImportRejection? CheckUpgrade(QualityModel candidateQuality, MovieFile? existingFile, Movie movie, string filePath)
    {
        if (existingFile == null)
        {
            _logger.LogDebug("No existing file for movie {MovieId}, allowing import", movie.Id);
            return null; // No existing file = always allow
        }

        if (existingFile.Quality == null)
        {
            _logger.LogDebug("Existing file has no quality data, allowing import");
            return null; // Allow if existing file has no quality (legacy data)
        }

        // Check if candidate is an upgrade
        var isUpgrade = QualityComparer.IsUpgrade(existingFile.Quality, candidateQuality);

        if (!isUpgrade)
        {
            _logger.LogDebug(
                "Quality {CandidateQuality} is not an upgrade over existing {ExistingQuality} for {FilePath}",
                candidateQuality.Quality.Name,
                existingFile.Quality.Quality.Name,
                filePath.SanitizeForLog());

            return new ImportRejection(
                ImportRejectionReason.NotQualityUpgrade,
                $"Quality {candidateQuality.Quality.Name} is not an upgrade over existing {existingFile.Quality.Quality.Name}");
        }

        _logger.LogDebug(
            "Quality {CandidateQuality} is an upgrade over {ExistingQuality} for {FilePath}",
            candidateQuality.Quality.Name,
            existingFile.Quality.Quality.Name,
            filePath.SanitizeForLog());

        return null; // Upgrade allowed
    }
}
