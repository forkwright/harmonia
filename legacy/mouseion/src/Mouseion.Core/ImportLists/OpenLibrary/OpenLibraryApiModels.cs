// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.ImportLists.OpenLibrary;

/// <summary>
/// Response from OpenLibrary reading log API.
/// Endpoint: /people/{username}/books/{shelf}.json?limit=50&offset=0
/// Shelves: want-to-read, currently-reading, already-read
/// </summary>
public class OpenLibraryReadingLogResponse
{
    [JsonPropertyName("page")]
    public OpenLibraryReadingLogPage Page { get; set; } = new();

    [JsonPropertyName("reading_log_entries")]
    public List<OpenLibraryReadingLogEntry> ReadingLogEntries { get; set; } = new();
}

public class OpenLibraryReadingLogPage
{
    [JsonPropertyName("totalCount")]
    public int TotalCount { get; set; }

    /// <summary>
    /// Seems unused in some responses; totalCount is authoritative.
    /// </summary>
    [JsonPropertyName("count")]
    public int Count { get; set; }
}

public class OpenLibraryReadingLogEntry
{
    [JsonPropertyName("work")]
    public OpenLibraryWork Work { get; set; } = new();

    [JsonPropertyName("logged_edition")]
    public string? LoggedEdition { get; set; }

    [JsonPropertyName("logged_date")]
    public string? LoggedDate { get; set; }
}

public class OpenLibraryWork
{
    /// <summary>
    /// Work key, e.g., "/works/OL45883W"
    /// </summary>
    [JsonPropertyName("key")]
    public string Key { get; set; } = string.Empty;

    [JsonPropertyName("title")]
    public string Title { get; set; } = string.Empty;

    [JsonPropertyName("author_names")]
    public List<string> AuthorNames { get; set; } = new();

    [JsonPropertyName("author_keys")]
    public List<string> AuthorKeys { get; set; } = new();

    [JsonPropertyName("first_publish_year")]
    public int? FirstPublishYear { get; set; }

    [JsonPropertyName("lending_edition_s")]
    public string? LendingEdition { get; set; }

    [JsonPropertyName("cover_edition_key")]
    public string? CoverEditionKey { get; set; }

    [JsonPropertyName("cover_id")]
    public int? CoverId { get; set; }

    /// <summary>
    /// ISBNs associated with this work (may contain multiple editions).
    /// </summary>
    [JsonPropertyName("isbn")]
    public List<string> Isbns { get; set; } = new();

    /// <summary>
    /// Number of editions of this work.
    /// </summary>
    [JsonPropertyName("edition_count")]
    public int? EditionCount { get; set; }
}

/// <summary>
/// Response from OpenLibrary ratings API.
/// Endpoint: /works/{workId}/ratings.json
/// </summary>
public class OpenLibraryRatingsResponse
{
    [JsonPropertyName("summary")]
    public OpenLibraryRatingSummary Summary { get; set; } = new();
}

public class OpenLibraryRatingSummary
{
    [JsonPropertyName("average")]
    public double? Average { get; set; }

    [JsonPropertyName("count")]
    public int Count { get; set; }
}
