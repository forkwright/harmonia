// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Api.Lookup;

public class BookLookupResource
{
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? ForeignBookId { get; set; }
    public string? OpenLibraryId { get; set; }
    public string? GoodreadsId { get; set; }
    public string? GoogleBooksId { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Description { get; set; }
    public int? PageCount { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; } = new();
    public DateTime? ReleaseDate { get; set; }
}

public class AudiobookLookupResource
{
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? ForeignAudiobookId { get; set; }
    public string? AudnexusId { get; set; }
    public string? AudibleId { get; set; }
    public string? Asin { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Description { get; set; }
    public string? Narrator { get; set; }
    public List<string> Narrators { get; set; } = new();
    public int? DurationMinutes { get; set; }
    public bool IsAbridged { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; } = new();
    public DateTime? ReleaseDate { get; set; }
}

public class TorrentLookupResource
{
    public string TorrentId { get; set; } = null!;
    public string Title { get; set; } = null!;
    public string? Author { get; set; }
    public string? Category { get; set; }
    public long Size { get; set; }
    public int Seeders { get; set; }
    public int Leechers { get; set; }
    public DateTime? PublishDate { get; set; }
    public string? DownloadUrl { get; set; }
    public string? InfoUrl { get; set; }
    public bool IsFreeleech { get; set; }
}
