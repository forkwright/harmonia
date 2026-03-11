// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.AniList;

public class AniListSettings : ImportListSettingsBase
{
    public AniListSettings()
    {
        BaseUrl = "https://graphql.anilist.co";
    }

    public string AccessToken { get; set; } = string.Empty;
    public string Username { get; set; } = string.Empty;

    public bool ImportAnimeList { get; set; } = true;
    public bool ImportMangaList { get; set; } = true;

    /// <summary>
    /// Filter by status: CURRENT, PLANNING, COMPLETED, DROPPED, PAUSED, REPEATING.
    /// Empty = import all statuses.
    /// </summary>
    public List<string> StatusFilter { get; set; } = new();

    /// <summary>
    /// AniList OAuth is optional — public profiles can be fetched by username alone.
    /// Token needed for private lists.
    /// </summary>
    public bool HasValidCredentials => !string.IsNullOrEmpty(Username) || !string.IsNullOrEmpty(AccessToken);
}
