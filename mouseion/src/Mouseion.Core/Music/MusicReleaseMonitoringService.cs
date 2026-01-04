// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Core.Music;

public class MusicReleaseMonitoringService : IMusicReleaseMonitoringService
{
    private readonly IArtistRepository _artistRepository;
    private readonly IAlbumRepository _albumRepository;
    private readonly IProvideMusicInfo _musicInfoProvider;
    private readonly ILogger<MusicReleaseMonitoringService> _logger;

    public MusicReleaseMonitoringService(
        IArtistRepository artistRepository,
        IAlbumRepository albumRepository,
        IProvideMusicInfo musicInfoProvider,
        ILogger<MusicReleaseMonitoringService> logger)
    {
        _artistRepository = artistRepository;
        _albumRepository = albumRepository;
        _musicInfoProvider = musicInfoProvider;
        _logger = logger;
    }

    public async Task<List<NewRelease>> CheckForNewReleasesAsync(int artistId, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Checking for new releases for artist ID: {ArtistId}", artistId);

            var artist = await _artistRepository.GetAsync(artistId, ct).ConfigureAwait(false);
            if (artist == null || string.IsNullOrEmpty(artist.MusicBrainzId))
            {
                _logger.LogWarning("Artist {ArtistId} not found or missing MusicBrainz ID", artistId);
                return new List<NewRelease>();
            }

            return await CheckForNewReleasesAsync(artist.MusicBrainzId, ct).ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error checking for new releases for artist {ArtistId}", artistId);
            return new List<NewRelease>();
        }
    }

    public async Task<List<NewRelease>> CheckForNewReleasesAsync(string artistMbid, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Checking for new releases for artist MBID: {ArtistMbid}", artistMbid);

            // Query MusicBrainz for all releases by this artist
            var albums = await _musicInfoProvider.GetAlbumsByArtistAsync(artistMbid, ct).ConfigureAwait(false);
            if (albums == null || albums.Count == 0)
            {
                _logger.LogWarning("No releases found for artist {ArtistMbid}", artistMbid);
                return new List<NewRelease>();
            }

            // Get existing albums from database
            var artist = await _artistRepository.FindByMusicBrainzIdAsync(artistMbid, ct).ConfigureAwait(false);
            if (artist == null)
            {
                _logger.LogWarning("Artist {ArtistMbid} not found in database", artistMbid);
                return new List<NewRelease>();
            }

            var existingAlbums = await _albumRepository.GetByArtistIdAsync(artist.Id, ct).ConfigureAwait(false);
            var existingMbids = existingAlbums
                .Where(a => !string.IsNullOrEmpty(a.ReleaseGroupMbid))
                .Select(a => a.ReleaseGroupMbid!)
                .ToHashSet(StringComparer.OrdinalIgnoreCase);

            // Find new releases (albums not in database)
            var newReleases = new List<NewRelease>();
            foreach (var album in albums)
            {
                if (!string.IsNullOrEmpty(album.ReleaseGroupMbid) &&
                    !existingMbids.Contains(album.ReleaseGroupMbid))
                {
                    var newRelease = new NewRelease
                    {
                        Title = album.Title ?? "Unknown",
                        ReleaseGroupMbid = album.ReleaseGroupMbid,
                        ReleaseMbid = album.MusicBrainzId,
                        ReleaseDate = album.ReleaseDate,
                        AlbumType = album.AlbumType,
                        TrackCount = album.TrackCount
                    };

                    newReleases.Add(newRelease);
                }
            }

            _logger.LogInformation("Found {Count} new releases for artist {ArtistMbid}", newReleases.Count, artistMbid);
            return newReleases;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error checking for new releases for artist {ArtistMbid}", artistMbid);
            return new List<NewRelease>();
        }
    }
}
