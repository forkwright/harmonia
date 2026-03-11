// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.ImportLists.ListenBrainz;

// ListenBrainz API response models
// Docs: https://listenbrainz.readthedocs.io/en/latest/users/api/

/// <summary>
/// Response from /1/user/{username}/listens
/// </summary>
public class ListenBrainzListensResponse
{
    [JsonPropertyName("payload")]
    public ListenBrainzListensPayload Payload { get; set; } = new();
}

public class ListenBrainzListensPayload
{
    [JsonPropertyName("count")]
    public int Count { get; set; }

    [JsonPropertyName("user_id")]
    public string UserId { get; set; } = string.Empty;

    [JsonPropertyName("listens")]
    public List<ListenBrainzListen> Listens { get; set; } = new();

    /// <summary>
    /// Oldest listen timestamp in the response. Use as max_ts for pagination.
    /// </summary>
    [JsonPropertyName("oldest_listen_ts")]
    public long? OldestListenTs { get; set; }

    /// <summary>
    /// Latest listen timestamp in the response.
    /// </summary>
    [JsonPropertyName("latest_listen_ts")]
    public long? LatestListenTs { get; set; }
}

public class ListenBrainzListen
{
    [JsonPropertyName("listened_at")]
    public long ListenedAt { get; set; }

    [JsonPropertyName("recording_msid")]
    public string? RecordingMsid { get; set; }

    [JsonPropertyName("track_metadata")]
    public ListenBrainzTrackMetadata TrackMetadata { get; set; } = new();
}

public class ListenBrainzTrackMetadata
{
    [JsonPropertyName("artist_name")]
    public string ArtistName { get; set; } = string.Empty;

    [JsonPropertyName("track_name")]
    public string TrackName { get; set; } = string.Empty;

    [JsonPropertyName("release_name")]
    public string? ReleaseName { get; set; }

    [JsonPropertyName("additional_info")]
    public ListenBrainzAdditionalInfo? AdditionalInfo { get; set; }

    /// <summary>
    /// MBID mapping (when available). ListenBrainz maps listens to MusicBrainz IDs.
    /// </summary>
    [JsonPropertyName("mbid_mapping")]
    public ListenBrainzMbidMapping? MbidMapping { get; set; }
}

public class ListenBrainzAdditionalInfo
{
    [JsonPropertyName("recording_mbid")]
    public string? RecordingMbid { get; set; }

    [JsonPropertyName("release_mbid")]
    public string? ReleaseMbid { get; set; }

    [JsonPropertyName("artist_mbids")]
    public List<string>? ArtistMbids { get; set; }

    [JsonPropertyName("release_group_mbid")]
    public string? ReleaseGroupMbid { get; set; }

    [JsonPropertyName("duration_ms")]
    public int? DurationMs { get; set; }
}

public class ListenBrainzMbidMapping
{
    [JsonPropertyName("recording_mbid")]
    public string? RecordingMbid { get; set; }

    [JsonPropertyName("release_mbid")]
    public string? ReleaseMbid { get; set; }

    [JsonPropertyName("artist_mbids")]
    public List<string>? ArtistMbids { get; set; }

    [JsonPropertyName("recording_name")]
    public string? RecordingName { get; set; }

    [JsonPropertyName("artist_credit_name")]
    public string? ArtistCreditName { get; set; }
}

/// <summary>
/// Response from /1/feedback/user/{username}/get-feedback
/// </summary>
public class ListenBrainzFeedbackResponse
{
    [JsonPropertyName("feedback")]
    public List<ListenBrainzFeedback> Feedback { get; set; } = new();

    [JsonPropertyName("count")]
    public int Count { get; set; }

    [JsonPropertyName("total_count")]
    public int TotalCount { get; set; }

    [JsonPropertyName("offset")]
    public int Offset { get; set; }
}

public class ListenBrainzFeedback
{
    /// <summary>
    /// 1 = love, -1 = hate, 0 = neutral
    /// </summary>
    [JsonPropertyName("score")]
    public int Score { get; set; }

    [JsonPropertyName("recording_mbid")]
    public string? RecordingMbid { get; set; }

    [JsonPropertyName("recording_msid")]
    public string? RecordingMsid { get; set; }

    [JsonPropertyName("track_metadata")]
    public ListenBrainzTrackMetadata? TrackMetadata { get; set; }

    [JsonPropertyName("created")]
    public long Created { get; set; }
}
