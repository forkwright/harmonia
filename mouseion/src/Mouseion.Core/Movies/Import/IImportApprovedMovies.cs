// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies.Import;

public interface IImportApprovedMovies
{
    Task<List<MovieImportResult>> ImportAsync(List<MovieImportDecision> decisions, CancellationToken ct = default);
    List<MovieImportResult> Import(List<MovieImportDecision> decisions);
}

public class MovieImportResult
{
    public MovieImportDecision Decision { get; }
    public bool Success { get; }
    public string? ErrorMessage { get; }

    public MovieImportResult(MovieImportDecision decision, bool success = true, string? errorMessage = null)
    {
        Decision = decision;
        Success = success;
        ErrorMessage = errorMessage;
    }
}
