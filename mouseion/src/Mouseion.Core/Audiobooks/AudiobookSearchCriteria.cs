// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Audiobooks;

public class AudiobookSearchCriteria
{
    public string? Title { get; set; }
    public string? Author { get; set; }
    public string? Narrator { get; set; }
    public string? Asin { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? AudnexusId { get; set; }
    public string? AudibleId { get; set; }
    public int? Year { get; set; }
    public string? Publisher { get; set; }
    public List<string>? Genres { get; set; }
    public bool? IsAbridged { get; set; }

    public bool HasSearchTerms()
    {
        return !string.IsNullOrWhiteSpace(Title) ||
               !string.IsNullOrWhiteSpace(Author) ||
               !string.IsNullOrWhiteSpace(Narrator) ||
               !string.IsNullOrWhiteSpace(Asin) ||
               !string.IsNullOrWhiteSpace(Isbn) ||
               !string.IsNullOrWhiteSpace(Isbn13) ||
               !string.IsNullOrWhiteSpace(AudnexusId) ||
               !string.IsNullOrWhiteSpace(AudibleId);
    }
}
