// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaFiles.Import;

namespace Mouseion.Core.Movies.Import.Specifications;

public interface IMovieImportSpecification
{
    Task<ImportRejection?> IsSatisfiedByAsync(string filePath, Movie movie, CancellationToken ct = default);
    ImportRejection? IsSatisfiedBy(string filePath, Movie movie);
}
