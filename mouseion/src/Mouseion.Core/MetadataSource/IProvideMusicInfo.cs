// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Music;

namespace Mouseion.Core.MetadataSource;

public interface IProvideMusicInfo
{
    Task<Artist?> GetArtistByMusicBrainzIdAsync(string mbid, CancellationToken ct = default);
    Task<Album?> GetAlbumByMusicBrainzIdAsync(string mbid, CancellationToken ct = default);
    Task<List<Artist>> SearchArtistsByNameAsync(string name, CancellationToken ct = default);
    Task<List<Album>> SearchAlbumsByTitleAsync(string title, CancellationToken ct = default);
    Task<List<Album>> GetAlbumsByArtistAsync(string artistMbid, CancellationToken ct = default);
    Task<List<Album>> GetTrendingAlbumsAsync(CancellationToken ct = default);
    Task<List<Album>> GetPopularAlbumsAsync(CancellationToken ct = default);

    Artist? GetArtistByMusicBrainzId(string mbid);
    Album? GetAlbumByMusicBrainzId(string mbid);
    List<Artist> SearchArtistsByName(string name);
    List<Album> SearchAlbumsByTitle(string title);
    List<Album> GetAlbumsByArtist(string artistMbid);
    List<Album> GetTrendingAlbums();
    List<Album> GetPopularAlbums();
}
