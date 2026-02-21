// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.Trakt;

public class TraktSettings : ImportListSettingsBase
{
    public TraktSettings()
    {
        BaseUrl = "https://api.trakt.tv";
    }

    public string ClientId { get; set; } = string.Empty;
    public string ClientSecret { get; set; } = string.Empty;
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public DateTime? TokenExpiresAt { get; set; }

    // Import scope configuration
    public bool ImportWatchlist { get; set; } = true;
    public bool ImportCollection { get; set; } = true;
    public bool ImportWatchHistory { get; set; } = true;
    public bool ImportRatings { get; set; } = true;
    public string? CustomListSlug { get; set; }

    // Sync configuration
    public DateTime? LastSyncedAt { get; set; }
    public string TraktUsername { get; set; } = string.Empty;

    public bool HasValidToken => !string.IsNullOrEmpty(AccessToken)
        && (!TokenExpiresAt.HasValue || TokenExpiresAt.Value > DateTime.UtcNow);
}
