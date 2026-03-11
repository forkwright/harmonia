// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Audiobooks;

public class AudiobookMetadata
{
    public AudiobookMetadata()
    {
        Genres = new List<string>();
        Narrators = new List<string>();
    }

    public string? Description { get; set; }
    public string? ForeignAudiobookId { get; set; }
    public string? AudnexusId { get; set; }
    public string? AudibleId { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Asin { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; }

    // Audiobook-specific fields
    public string? Narrator { get; set; }
    public List<string> Narrators { get; set; }
    public int? DurationMinutes { get; set; }
    public bool IsAbridged { get; set; }
    public int? SeriesPosition { get; set; }
    public int? BookId { get; set; }  // Link to related book edition
}
