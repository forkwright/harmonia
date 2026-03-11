// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.ListenBrainz;

public class ListenBrainzSettings : ImportListSettingsBase
{
    public ListenBrainzSettings()
    {
        BaseUrl = "https://api.listenbrainz.org/1";
    }

    /// <summary>
    /// ListenBrainz user token. Found at https://listenbrainz.org/settings/.
    /// Required for accessing listen history and feedback.
    /// </summary>
    public string UserToken { get; set; } = string.Empty;

    /// <summary>
    /// ListenBrainz username. Used for public API endpoints.
    /// </summary>
    public string Username { get; set; } = string.Empty;

    /// <summary>
    /// Import listen history (scrobbles with MusicBrainz IDs).
    /// </summary>
    public bool ImportListens { get; set; } = true;

    /// <summary>
    /// Import user feedback (love/hate). Love maps to rating 10, hate maps to rating 1.
    /// </summary>
    public bool ImportFeedback { get; set; } = true;

    /// <summary>
    /// Maximum listens to import per sync. ListenBrainz returns up to 100 per request.
    /// </summary>
    public int MaxListens { get; set; } = 10000;

    /// <summary>
    /// Last sync timestamp (Unix epoch). ListenBrainz supports min_ts/max_ts for incremental.
    /// </summary>
    public long? LastSyncedTimestamp { get; set; }

    public bool IsConfigured => !string.IsNullOrEmpty(Username);
}
