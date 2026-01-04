// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists;

public class ImportListItem
{
    public int ListId { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }

    // Movie identifiers
    public int TmdbId { get; set; }
    public string? ImdbId { get; set; }

    // TV identifiers
    public int TvdbId { get; set; }

    // Music identifiers
    public Guid MusicBrainzId { get; set; }
    public string? Artist { get; set; }
    public string? Album { get; set; }

    // Book identifiers
    public long GoodreadsId { get; set; }
    public string? Isbn { get; set; }
    public string? Author { get; set; }

    // Audiobook identifiers
    public string? AudibleId { get; set; }
    public string? Asin { get; set; }
    public string? Narrator { get; set; }

    // Podcast identifiers
    public string? PodcastGuid { get; set; }
    public string? FeedUrl { get; set; }

    public string? ReleaseDate { get; set; }
}
