// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.SmartPlaylists;

public interface ISmartPlaylistService
{
    Task<IEnumerable<SmartPlaylist>> GetAllAsync(CancellationToken ct = default);
    Task<SmartPlaylist?> GetAsync(int id, CancellationToken ct = default);
    Task<SmartPlaylist> CreateAsync(SmartPlaylist playlist, CancellationToken ct = default);
    Task<SmartPlaylist> UpdateAsync(SmartPlaylist playlist, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task<SmartPlaylist> RefreshAsync(int id, CancellationToken ct = default);
    Task<IEnumerable<SmartPlaylistTrack>> GetTracksAsync(int id, CancellationToken ct = default);
}
