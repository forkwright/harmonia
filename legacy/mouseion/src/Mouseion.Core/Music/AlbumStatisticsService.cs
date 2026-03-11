// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Music;

public class AlbumStatistics
{
    public int AlbumId { get; set; }
    public int TrackCount { get; set; }
    public int TrackFileCount { get; set; }
    public long SizeOnDisk { get; set; }
    public decimal PercentOfTracks { get; set; }
}

public interface IAlbumStatisticsService
{
    Task<AlbumStatistics> GetStatisticsAsync(int albumId, CancellationToken ct = default);
    AlbumStatistics GetStatistics(int albumId);
}

public class AlbumStatisticsService : IAlbumStatisticsService
{
    private readonly ITrackRepository _trackRepository;
    private readonly IMusicFileRepository _musicFileRepository;

    public AlbumStatisticsService(
        ITrackRepository trackRepository,
        IMusicFileRepository musicFileRepository)
    {
        _trackRepository = trackRepository;
        _musicFileRepository = musicFileRepository;
    }

    public async Task<AlbumStatistics> GetStatisticsAsync(int albumId, CancellationToken ct = default)
    {
        var tracks = await _trackRepository.GetByAlbumIdAsync(albumId, ct).ConfigureAwait(false);

        var stats = new AlbumStatistics
        {
            AlbumId = albumId,
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

        stats.PercentOfTracks = stats.TrackCount > 0
            ? (decimal)stats.TrackFileCount / stats.TrackCount * 100
            : 0;

        return stats;
    }

    public AlbumStatistics GetStatistics(int albumId)
    {
        var tracks = _trackRepository.GetByAlbumId(albumId);

        var stats = new AlbumStatistics
        {
            AlbumId = albumId,
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

        stats.PercentOfTracks = stats.TrackCount > 0
            ? (decimal)stats.TrackFileCount / stats.TrackCount * 100
            : 0;

        return stats;
    }
}
