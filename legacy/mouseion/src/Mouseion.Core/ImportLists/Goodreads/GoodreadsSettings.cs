// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.Goodreads;

public class GoodreadsSettings : ImportListSettingsBase
{
    public GoodreadsSettings()
    {
        BaseUrl = "https://www.goodreads.com";
    }

    /// <summary>
    /// Goodreads user ID (numeric). Found in profile URL: goodreads.com/user/show/{UserId}
    /// </summary>
    public string UserId { get; set; } = string.Empty;

    /// <summary>
    /// Shelves to import. Goodreads has 3 default shelves (read, currently-reading, to-read)
    /// plus user-created custom shelves. Each shelf has an RSS feed.
    /// </summary>
    public List<string> Shelves { get; set; } = new() { "read", "currently-reading", "to-read" };

    /// <summary>
    /// Map shelf names to import behavior. Shelves not in this map are treated as "to-read" (monitored).
    /// </summary>
    public Dictionary<string, GoodreadsShelfMapping> ShelfMappings { get; set; } = new()
    {
        ["read"] = GoodreadsShelfMapping.Completed,
        ["currently-reading"] = GoodreadsShelfMapping.InProgress,
        ["to-read"] = GoodreadsShelfMapping.Monitored
    };

    /// <summary>
    /// Import user ratings from Goodreads (1-5 stars → scaled to 1-10).
    /// </summary>
    public bool ImportRatings { get; set; } = true;

    /// <summary>
    /// Detect audiobooks from Goodreads edition format/binding field.
    /// When true, editions marked "Audio CD", "Audiobook", "Audio Cassette" get MediaType.Audiobook.
    /// </summary>
    public bool DetectAudiobooks { get; set; } = true;

    /// <summary>
    /// Last successful sync timestamp for incremental import.
    /// </summary>
    public DateTime? LastSyncedAt { get; set; }

    public bool IsConfigured => !string.IsNullOrEmpty(UserId);
}

public enum GoodreadsShelfMapping
{
    Completed,
    InProgress,
    Monitored,
    Ignored
}
