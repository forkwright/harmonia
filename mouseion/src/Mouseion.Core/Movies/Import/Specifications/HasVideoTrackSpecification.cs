// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaFiles.Import;

namespace Mouseion.Core.Movies.Import.Specifications;

public class HasVideoTrackSpecification : IMovieImportSpecification
{
    private readonly ILogger<HasVideoTrackSpecification> _logger;

    public HasVideoTrackSpecification(ILogger<HasVideoTrackSpecification> logger)
    {
        _logger = logger;
    }

    public Task<ImportRejection?> IsSatisfiedByAsync(string filePath, Movie movie, CancellationToken ct = default)
    {
        // TODO: Implement video track detection when MediaAnalyzer supports it
        _logger.LogDebug("Skipping video track check for {FilePath} - not yet implemented", filePath);
        return Task.FromResult<ImportRejection?>(null);
    }

    public ImportRejection? IsSatisfiedBy(string filePath, Movie movie)
    {
        // TODO: Implement video track detection when MediaAnalyzer supports it
        _logger.LogDebug("Skipping video track check for {FilePath} - not yet implemented", filePath);
        return null;
    }
}
