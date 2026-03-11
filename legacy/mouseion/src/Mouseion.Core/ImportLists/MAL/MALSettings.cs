// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.MAL;

public class MALSettings : ImportListSettingsBase
{
    public MALSettings()
    {
        BaseUrl = "https://api.myanimelist.net/v2";
    }

    public string ClientId { get; set; } = string.Empty;
    public string ClientSecret { get; set; } = string.Empty;
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public DateTime? TokenExpiresAt { get; set; }

    public bool ImportAnimeList { get; set; } = true;
    public bool ImportMangaList { get; set; } = true;

    /// <summary>
    /// Filter by status: watching, completed, on_hold, dropped, plan_to_watch (anime)
    /// or reading, completed, on_hold, dropped, plan_to_read (manga)
    /// Empty = import all statuses.
    /// </summary>
    public List<string> StatusFilter { get; set; } = new();

    public bool HasValidToken => !string.IsNullOrEmpty(AccessToken) &&
        (!TokenExpiresAt.HasValue || TokenExpiresAt.Value > DateTime.UtcNow);
}
