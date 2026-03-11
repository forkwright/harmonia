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

public class MinimumQualitySpecification : IMovieImportSpecification
{
    private readonly ILogger<MinimumQualitySpecification> _logger;

    public MinimumQualitySpecification(ILogger<MinimumQualitySpecification> logger)
    {
        _logger = logger;
    }

    public Task<ImportRejection?> IsSatisfiedByAsync(string filePath, Movie movie, CancellationToken ct = default)
    {
        return Task.FromResult(CheckMinimumQuality(filePath, movie));
    }

    public ImportRejection? IsSatisfiedBy(string filePath, Movie movie)
    {
        return CheckMinimumQuality(filePath, movie);
    }

    private ImportRejection? CheckMinimumQuality(string filePath, Movie movie)
    {
        // Parse quality from candidate file
        var candidateQuality = QualityParser.ParseQuality(filePath, _logger);

        if (candidateQuality.Quality == Quality.Unknown)
        {
            _logger.LogDebug("Cannot determine quality for {FilePath}, rejecting import for safety", filePath.SanitizeForLog());
            return new ImportRejection(ImportRejectionReason.UnableToParse, "Cannot determine quality from filename");
        }

        // Quality profiles not yet implemented - allow all known qualities
        // TODO: When QualityProfiles are implemented, retrieve minimum from movie.QualityProfileId
        // and enforce with: QualityComparer.MeetsMinimum(candidateQuality, minimumQuality)

        _logger.LogDebug(
            "Quality {Quality} meets requirements for {FilePath} (quality profiles not yet enforced)",
            candidateQuality.Quality.Name,
            filePath.SanitizeForLog());

        return null; // Allow import
    }
}
