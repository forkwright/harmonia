// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaFiles.Import;

namespace Mouseion.Core.Movies.Import;

public class MovieImportDecision
{
    public string FilePath { get; }
    public Movie Movie { get; }
    public List<ImportRejection> Rejections { get; }
    public bool Approved => Rejections.Count == 0;

    public MovieImportDecision(string filePath, Movie movie)
    {
        FilePath = filePath;
        Movie = movie;
        Rejections = new List<ImportRejection>();
    }

    public MovieImportDecision(string filePath, Movie movie, params ImportRejection[] rejections)
    {
        FilePath = filePath;
        Movie = movie;
        Rejections = rejections.ToList();
    }

    public void AddRejection(ImportRejection rejection)
    {
        Rejections.Add(rejection);
    }
}
