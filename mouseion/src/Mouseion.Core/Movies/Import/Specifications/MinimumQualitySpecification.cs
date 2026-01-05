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
        // TODO: Implement minimum quality check when QualityDetector and MinimumQualityId are available
        _logger.LogDebug("Skipping minimum quality check for {FilePath} - not yet implemented", filePath.SanitizeForLog());
        return Task.FromResult<ImportRejection?>(null);
    }

    public ImportRejection? IsSatisfiedBy(string filePath, Movie movie)
    {
        // TODO: Implement minimum quality check when QualityDetector and MinimumQualityId are available
        _logger.LogDebug("Skipping minimum quality check for {FilePath} - not yet implemented", filePath.SanitizeForLog());
        return null;
    }
}
