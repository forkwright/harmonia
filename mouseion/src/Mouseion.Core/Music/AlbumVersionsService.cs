// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Music;

public interface IAlbumVersionsService
{
    Task<AlbumVersionsResult?> GetVersionsAsync(int albumId, CancellationToken ct = default);
    AlbumVersionsResult? GetVersions(int albumId);
}

public class AlbumVersionsService : IAlbumVersionsService
{
    private readonly IAlbumRepository _albumRepository;

    public AlbumVersionsService(IAlbumRepository albumRepository)
    {
        _albumRepository = albumRepository;
    }

    public async Task<AlbumVersionsResult?> GetVersionsAsync(int albumId, CancellationToken ct = default)
    {
        var album = await _albumRepository.GetAsync(albumId, ct).ConfigureAwait(false);
        if (album == null)
        {
            return null;
        }

        if (string.IsNullOrEmpty(album.ReleaseGroupMbid))
        {
            return new AlbumVersionsResult
            {
                Canonical = album,
                Versions = new List<Album>()
            };
        }

        var allVersions = await _albumRepository.GetVersionsAsync(album.ReleaseGroupMbid, ct).ConfigureAwait(false);
        var otherVersions = allVersions.Where(a => a.Id != albumId).ToList();

        return new AlbumVersionsResult
        {
            Canonical = album,
            Versions = otherVersions
        };
    }

    public AlbumVersionsResult? GetVersions(int albumId)
    {
        var album = _albumRepository.Get(albumId);
        if (album == null)
        {
            return null;
        }

        if (string.IsNullOrEmpty(album.ReleaseGroupMbid))
        {
            return new AlbumVersionsResult
            {
                Canonical = album,
                Versions = new List<Album>()
            };
        }

        var allVersions = _albumRepository.GetVersions(album.ReleaseGroupMbid);
        var otherVersions = allVersions.Where(a => a.Id != albumId).ToList();

        return new AlbumVersionsResult
        {
            Canonical = album,
            Versions = otherVersions
        };
    }
}

public class AlbumVersionsResult
{
    public Album Canonical { get; set; } = null!;
    public List<Album> Versions { get; set; } = new();
}
