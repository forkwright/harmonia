// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.OpenLibrary;

public class OpenLibrarySettings : ImportListSettingsBase
{
    public OpenLibrarySettings()
    {
        BaseUrl = "https://openlibrary.org";
    }

    /// <summary>
    /// OpenLibrary username. Reading log is public at /people/{Username}/books.
    /// </summary>
    public string Username { get; set; } = string.Empty;

    /// <summary>
    /// Which reading log shelves to import.
    /// OpenLibrary has 3 shelves: want-to-read (1), currently-reading (2), already-read (3).
    /// </summary>
    public bool ImportWantToRead { get; set; } = true;
    public bool ImportCurrentlyReading { get; set; } = true;
    public bool ImportAlreadyRead { get; set; } = true;

    /// <summary>
    /// Import user ratings from OpenLibrary (1-5 stars → scaled to 1-10).
    /// Ratings are fetched separately from the reading log.
    /// </summary>
    public bool ImportRatings { get; set; } = true;

    /// <summary>
    /// Last successful sync timestamp.
    /// </summary>
    public DateTime? LastSyncedAt { get; set; }

    public bool IsConfigured => !string.IsNullOrEmpty(Username);
}
