// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Music;

public class ArtistStatistics
{
    public int ArtistId { get; set; }
    public int AlbumCount { get; set; }
    public int TrackCount { get; set; }
    public int TrackFileCount { get; set; }
    public long SizeOnDisk { get; set; }
}

public interface IArtistStatisticsService
{
    Task<ArtistStatistics> GetStatisticsAsync(int artistId, CancellationToken ct = default);
    ArtistStatistics GetStatistics(int artistId);
}

public class ArtistStatisticsService : IArtistStatisticsService
{
    private readonly IAlbumRepository _albumRepository;
    private readonly ITrackRepository _trackRepository;
    private readonly IMusicFileRepository _musicFileRepository;

    public ArtistStatisticsService(
        IAlbumRepository albumRepository,
        ITrackRepository trackRepository,
        IMusicFileRepository musicFileRepository)
    {
        _albumRepository = albumRepository;
        _trackRepository = trackRepository;
        _musicFileRepository = musicFileRepository;
    }

    public async Task<ArtistStatistics> GetStatisticsAsync(int artistId, CancellationToken ct = default)
    {
        var albums = await _albumRepository.GetByArtistIdAsync(artistId, ct).ConfigureAwait(false);
        var tracks = await _trackRepository.GetByArtistIdAsync(artistId, ct).ConfigureAwait(false);

        var stats = new ArtistStatistics
        {
            ArtistId = artistId,
            AlbumCount = albums.Count,
            TrackCount = tracks.Count,
            TrackFileCount = 0,
            SizeOnDisk = 0
        };

        foreach (var track in tracks)
        {
            var files = await _musicFileRepository.GetByTrackIdAsync(track.Id, ct).ConfigureAwait(false);
            stats.TrackFileCount += files.Count;
            stats.SizeOnDisk += files.Sum(f => f.Size);
        }

        return stats;
    }

    public ArtistStatistics GetStatistics(int artistId)
    {
        var albums = _albumRepository.GetByArtistId(artistId);
        var tracks = _trackRepository.GetByArtistId(artistId);

        var stats = new ArtistStatistics
        {
            ArtistId = artistId,
            AlbumCount = albums.Count,
            TrackCount = tracks.Count,
            TrackFileCount = 0,
            SizeOnDisk = 0
        };

        foreach (var track in tracks)
        {
            var files = _musicFileRepository.GetByTrackId(track.Id);
            stats.TrackFileCount += files.Count;
            stats.SizeOnDisk += files.Sum(f => f.Size);
        }

        return stats;
    }
}
