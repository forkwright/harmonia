// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.Resources;

/// <summary>
/// Shared track resource used by Library and Track controllers.
/// </summary>
public class TrackResource
{
    public int Id { get; set; }
    public int? AlbumId { get; set; }
    public int? ArtistId { get; set; }
    public string Title { get; set; } = null!;
    public string? ForeignTrackId { get; set; }
    public string? MusicBrainzId { get; set; }
    public int TrackNumber { get; set; }
    public int DiscNumber { get; set; }
    public int? DurationSeconds { get; set; }
    public bool Explicit { get; set; }
    public MediaType MediaType { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string Path { get; set; } = string.Empty;
    public string RootFolderPath { get; set; } = string.Empty;
    public DateTime Added { get; set; }
    public List<int>? Tags { get; set; }
    public DateTime? LastSearchTime { get; set; }

    // Denormalized display fields
    public string? ArtistName { get; set; }
    public string? AlbumName { get; set; }
    public string? Genre { get; set; }

    // Audio quality fields (from associated MusicFile)
    public string? AudioFormat { get; set; }
    public int? SampleRate { get; set; }
    public int? BitDepth { get; set; }
    public int? Channels { get; set; }
    public int? Bitrate { get; set; }
    public long? FileSize { get; set; }

    // Cover art
    public string? CoverArtUrl { get; set; }

    // ReplayGain tags
    public double? ReplayGainTrackGain { get; set; }
    public double? ReplayGainAlbumGain { get; set; }
    public double? ReplayGainTrackPeak { get; set; }
    public double? ReplayGainAlbumPeak { get; set; }
}
