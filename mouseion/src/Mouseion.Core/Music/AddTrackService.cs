// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.Music;

public interface IAddTrackService
{
    Task<Track> AddTrackAsync(Track track, CancellationToken ct = default);
    Task<List<Track>> AddTracksAsync(List<Track> tracks, CancellationToken ct = default);

    Track AddTrack(Track track);
    List<Track> AddTracks(List<Track> tracks);
}

public class AddTrackService : IAddTrackService
{
    private readonly ITrackRepository _trackRepository;
    private readonly IAlbumRepository _albumRepository;
    private readonly IArtistRepository _artistRepository;
    private readonly ILogger<AddTrackService> _logger;

    public AddTrackService(
        ITrackRepository trackRepository,
        IAlbumRepository albumRepository,
        IArtistRepository artistRepository,
        ILogger<AddTrackService> logger)
    {
        _trackRepository = trackRepository;
        _albumRepository = albumRepository;
        _artistRepository = artistRepository;
        _logger = logger;
    }

    public async Task<Track> AddTrackAsync(Track track, CancellationToken ct = default)
    {
        ValidateTrack(track);

        if (track.ArtistId.HasValue)
        {
            var artist = await _artistRepository.FindAsync(track.ArtistId.Value, ct).ConfigureAwait(false);
            if (artist == null)
            {
                throw new ArgumentException($"Artist with ID {track.ArtistId.Value} not found", nameof(track));
            }
        }

        if (track.AlbumId.HasValue)
        {
            var album = await _albumRepository.FindAsync(track.AlbumId.Value, ct).ConfigureAwait(false);
            if (album == null)
            {
                throw new ArgumentException($"Album with ID {track.AlbumId.Value} not found", nameof(track));
            }
        }

        if (!string.IsNullOrWhiteSpace(track.ForeignTrackId))
        {
            var existing = await _trackRepository.FindByForeignIdAsync(track.ForeignTrackId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Track already exists: {TrackTitle} - ForeignId: {ForeignTrackId}",
                    track.Title.SanitizeForLog(), track.ForeignTrackId.SanitizeForLog());
                return existing;
            }
        }

        track.Added = DateTime.UtcNow;
        track.Monitored = true;

        var added = await _trackRepository.InsertAsync(track, ct).ConfigureAwait(false);
        _logger.LogInformation("Added track: {TrackNumber}. {TrackTitle} - Album ID: {AlbumId}, Artist ID: {ArtistId}",
            track.TrackNumber, added.Title.SanitizeForLog(), added.AlbumId, added.ArtistId);

        return added;
    }

    public Track AddTrack(Track track)
    {
        ValidateTrack(track);

        if (track.ArtistId.HasValue)
        {
            var artist = _artistRepository.Find(track.ArtistId.Value);
            if (artist == null)
            {
                throw new ArgumentException($"Artist with ID {track.ArtistId.Value} not found", nameof(track));
            }
        }

        if (track.AlbumId.HasValue)
        {
            var album = _albumRepository.Find(track.AlbumId.Value);
            if (album == null)
            {
                throw new ArgumentException($"Album with ID {track.AlbumId.Value} not found", nameof(track));
            }
        }

        if (!string.IsNullOrWhiteSpace(track.ForeignTrackId))
        {
            var existing = _trackRepository.FindByForeignId(track.ForeignTrackId);
            if (existing != null)
            {
                _logger.LogInformation("Track already exists: {TrackTitle} - ForeignId: {ForeignTrackId}",
                    track.Title.SanitizeForLog(), track.ForeignTrackId.SanitizeForLog());
                return existing;
            }
        }

        track.Added = DateTime.UtcNow;
        track.Monitored = true;

        var added = _trackRepository.Insert(track);
        _logger.LogInformation("Added track: {TrackNumber}. {TrackTitle} - Album ID: {AlbumId}, Artist ID: {ArtistId}",
            track.TrackNumber, added.Title.SanitizeForLog(), added.AlbumId, added.ArtistId);

        return added;
    }

    public async Task<List<Track>> AddTracksAsync(List<Track> tracks, CancellationToken ct = default)
    {
        var addedTracks = new List<Track>();

        foreach (var track in tracks)
        {
            try
            {
                var added = await AddTrackAsync(track, ct).ConfigureAwait(false);
                addedTracks.Add(added);
            }
            catch (ArgumentException ex)
            {
                _logger.LogError(ex, "Validation failed for track: {TrackTitle}", track.Title.SanitizeForLog());
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Error adding track: {TrackTitle}", track.Title.SanitizeForLog());
            }
        }

        return addedTracks;
    }

    public List<Track> AddTracks(List<Track> tracks)
    {
        var addedTracks = new List<Track>();

        foreach (var track in tracks)
        {
            try
            {
                var added = AddTrack(track);
                addedTracks.Add(added);
            }
            catch (ArgumentException ex)
            {
                _logger.LogError(ex, "Validation failed for track: {TrackTitle}", track.Title.SanitizeForLog());
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Error adding track: {TrackTitle}", track.Title.SanitizeForLog());
            }
        }

        return addedTracks;
    }

    private void ValidateTrack(Track track)
    {
        if (string.IsNullOrWhiteSpace(track.Title))
        {
            throw new ArgumentException("Track title is required", nameof(track));
        }

        if (track.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(track));
        }

        if (track.TrackNumber <= 0)
        {
            throw new ArgumentException("Track number must be greater than 0", nameof(track));
        }
    }
}
