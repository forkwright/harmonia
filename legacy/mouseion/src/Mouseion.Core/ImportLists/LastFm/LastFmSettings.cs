// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.LastFm;

public class LastFmSettings : ImportListSettingsBase
{
    public LastFmSettings()
    {
        BaseUrl = "https://ws.audioscrobbler.com/2.0/";
    }

    /// <summary>
    /// Last.fm API key. Free to obtain at https://www.last.fm/api/account/create.
    /// Read-only access doesn't require OAuth — API key is sufficient.
    /// </summary>
    public string ApiKey { get; set; } = string.Empty;

    /// <summary>
    /// Last.fm username to import from.
    /// </summary>
    public string Username { get; set; } = string.Empty;

    /// <summary>
    /// Import top artists (all-time library).
    /// </summary>
    public bool ImportTopArtists { get; set; } = true;

    /// <summary>
    /// Import top albums (all-time library).
    /// </summary>
    public bool ImportTopAlbums { get; set; } = true;

    /// <summary>
    /// Import recent tracks (scrobble history).
    /// Limited to last 200 pages (~200*50 = 10,000 tracks per sync).
    /// </summary>
    public bool ImportRecentTracks { get; set; } = true;

    /// <summary>
    /// Import loved tracks as high-rated items.
    /// </summary>
    public bool ImportLovedTracks { get; set; } = true;

    /// <summary>
    /// Time period for top artists/albums. Options: overall, 7day, 1month, 3month, 6month, 12month.
    /// </summary>
    public string TimePeriod { get; set; } = "overall";

    /// <summary>
    /// Maximum number of items per category (top artists, top albums).
    /// </summary>
    public int MaxItemsPerCategory { get; set; } = 500;

    /// <summary>
    /// Last successful sync timestamp for incremental scrobble import.
    /// </summary>
    public DateTime? LastSyncedAt { get; set; }

    public bool IsConfigured => !string.IsNullOrEmpty(ApiKey) && !string.IsNullOrEmpty(Username);
}
