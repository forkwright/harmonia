// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Authors;

public class Author : ModelBase
{
    public Author()
    {
        Tags = new HashSet<int>();
    }

    public string Name { get; set; } = null!;
    public string? SortName { get; set; }
    public string? Description { get; set; }
    public string? ForeignAuthorId { get; set; }  // OpenLibrary, Goodreads, etc.
    public bool Monitored { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
    public HashSet<int> Tags { get; set; }

    public override string ToString()
    {
        return Name;
    }
}
