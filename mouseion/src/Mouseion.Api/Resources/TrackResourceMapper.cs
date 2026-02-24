// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Music;

namespace Mouseion.Api.Resources;

/// <summary>
/// Shared mapper: Track + optional MusicFile → TrackResource.
/// Used by TrackController, LibraryController, and FavoritesController.
/// </summary>
public static class TrackResourceMapper
{
    public static TrackResource ToResource(Track track, MusicFile? musicFile = null)
    {
        var resource = new TrackResource
        {
            Id = track.Id,
            AlbumId = track.AlbumId,
            ArtistId = track.ArtistId,
            Title = track.Title,
            ForeignTrackId = track.ForeignTrackId,
            MusicBrainzId = track.MusicBrainzId,
            TrackNumber = track.TrackNumber,
            DiscNumber = track.DiscNumber,
            DurationSeconds = track.DurationSeconds,
            Explicit = track.Explicit,
            MediaType = track.MediaType,
            Monitored = track.Monitored,
            QualityProfileId = track.QualityProfileId,
            Path = track.Path,
            RootFolderPath = track.RootFolderPath,
            Added = track.Added,
            Tags = track.Tags?.ToList(),
            LastSearchTime = track.LastSearchTime,
            ArtistName = track.ArtistName,
            AlbumName = track.AlbumName,
            Genre = track.Genre,
            CoverArtUrl = track.AlbumId.HasValue ? $"/api/v3/albums/{track.AlbumId}/cover" : null,
        };

        if (musicFile != null)
        {
            resource.AudioFormat = musicFile.AudioFormat;
            resource.SampleRate = musicFile.SampleRate;
            resource.BitDepth = musicFile.BitDepth;
            resource.Channels = musicFile.Channels;
            resource.Bitrate = musicFile.Bitrate;
            resource.FileSize = musicFile.Size;
        }

        return resource;
    }

    public static async Task<TrackResource> ToResourceWithFileAsync(
        Track track, IMusicFileRepository musicFileRepository, CancellationToken ct = default)
    {
        var files = await musicFileRepository.GetByTrackIdAsync(track.Id, ct).ConfigureAwait(false);
        return ToResource(track, files.FirstOrDefault());
    }

    public static async Task<List<TrackResource>> ToResourcesWithFilesAsync(
        IEnumerable<Track> tracks, IMusicFileRepository musicFileRepository, CancellationToken ct = default)
    {
        var results = new List<TrackResource>();
        foreach (var track in tracks)
        {
            results.Add(await ToResourceWithFileAsync(track, musicFileRepository, ct).ConfigureAwait(false));
        }
        return results;
    }
}
