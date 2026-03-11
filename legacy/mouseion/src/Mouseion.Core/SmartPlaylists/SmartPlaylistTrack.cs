// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.SmartPlaylists;

/// <summary>
/// Join table entry linking a smart playlist to a track at a specific position.
/// </summary>
public class SmartPlaylistTrack : ModelBase
{
    public int SmartPlaylistId { get; set; }

    public int TrackId { get; set; }

    public int Position { get; set; }
}
