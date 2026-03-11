// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.SmartPlaylists;

public interface ISmartPlaylistRepository : IBasicRepository<SmartPlaylist>
{
    /// <summary>
    /// Get all tracks for a smart playlist, ordered by position.
    /// </summary>
    Task<IEnumerable<SmartPlaylistTrack>> GetTracksAsync(int smartPlaylistId, CancellationToken ct = default);

    /// <summary>
    /// Replace all tracks for a smart playlist atomically.
    /// </summary>
    Task SetTracksAsync(int smartPlaylistId, IList<SmartPlaylistTrack> tracks, CancellationToken ct = default);

    /// <summary>
    /// Get playlists whose LastRefreshed is older than the threshold (for refresh scheduling).
    /// </summary>
    Task<IEnumerable<SmartPlaylist>> GetStaleAsync(DateTime threshold, CancellationToken ct = default);
}
