// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.SmartPlaylists;

/// <summary>
/// A dynamic playlist that auto-populates from filter criteria against the track library.
/// </summary>
public class SmartPlaylist : ModelBase
{
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Serialized FilterRequest JSON — defines the criteria for track matching.
    /// </summary>
    public string FilterRequestJson { get; set; } = "{}";

    public int TrackCount { get; set; }

    public DateTime LastRefreshed { get; set; }

    public DateTime CreatedAt { get; set; }

    public DateTime UpdatedAt { get; set; }
}
