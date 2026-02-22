// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.Webhooks;

// ===== Jellyfin Webhook Payload =====
// Matches jellyfin-plugin-webhook output format

public class JellyfinWebhookPayload
{
    [JsonPropertyName("NotificationType")]
    public string? NotificationType { get; set; }

    [JsonPropertyName("ItemId")]
    public string? ItemId { get; set; }

    [JsonPropertyName("ItemType")]
    public string? ItemType { get; set; }

    [JsonPropertyName("Name")]
    public string? Name { get; set; }

    [JsonPropertyName("SeriesName")]
    public string? SeriesName { get; set; }

    [JsonPropertyName("SeasonNumber")]
    public int? SeasonNumber { get; set; }

    [JsonPropertyName("EpisodeNumber")]
    public int? EpisodeNumber { get; set; }

    [JsonPropertyName("Year")]
    public int? Year { get; set; }

    [JsonPropertyName("UserId")]
    public string? UserId { get; set; }

    [JsonPropertyName("UserName")]
    public string? UserName { get; set; }

    [JsonPropertyName("DeviceName")]
    public string? DeviceName { get; set; }

    [JsonPropertyName("ClientName")]
    public string? ClientName { get; set; }

    [JsonPropertyName("PlaybackPositionTicks")]
    public long? PlaybackPositionTicks { get; set; }

    [JsonPropertyName("RunTimeTicks")]
    public long? RunTimeTicks { get; set; }

    // Provider IDs
    [JsonPropertyName("Provider_tmdb")]
    public string? Provider_tmdb { get; set; }

    [JsonPropertyName("Provider_imdb")]
    public string? Provider_imdb { get; set; }

    [JsonPropertyName("Provider_tvdb")]
    public string? Provider_tvdb { get; set; }

    [JsonPropertyName("Provider_musicbrainzartist")]
    public string? Provider_musicbrainzartist { get; set; }
}

// ===== Emby Webhook Payload =====

public class EmbyWebhookPayload
{
    [JsonPropertyName("Event")]
    public string? Event { get; set; }

    [JsonPropertyName("Item")]
    public EmbyItem? Item { get; set; }

    [JsonPropertyName("User")]
    public EmbyUser? User { get; set; }

    [JsonPropertyName("Session")]
    public EmbySession? Session { get; set; }

    [JsonPropertyName("PlaybackInfo")]
    public EmbyPlaybackInfo? PlaybackInfo { get; set; }

    [JsonPropertyName("DeviceName")]
    public string? DeviceName { get; set; }
}

public class EmbyItem
{
    [JsonPropertyName("Id")]
    public string? Id { get; set; }

    [JsonPropertyName("Name")]
    public string? Name { get; set; }

    [JsonPropertyName("Type")]
    public string? Type { get; set; }

    [JsonPropertyName("ProductionYear")]
    public int? ProductionYear { get; set; }

    [JsonPropertyName("IndexNumber")]
    public int? IndexNumber { get; set; }

    [JsonPropertyName("ParentIndexNumber")]
    public int? ParentIndexNumber { get; set; }

    [JsonPropertyName("RunTimeTicks")]
    public long? RunTimeTicks { get; set; }

    [JsonPropertyName("ProviderIds")]
    public Dictionary<string, string>? ProviderIds { get; set; }
}

public class EmbyUser
{
    [JsonPropertyName("Id")]
    public string? Id { get; set; }

    [JsonPropertyName("Name")]
    public string? Name { get; set; }
}

public class EmbySession
{
    [JsonPropertyName("Id")]
    public string? Id { get; set; }

    [JsonPropertyName("DeviceName")]
    public string? DeviceName { get; set; }
}

public class EmbyPlaybackInfo
{
    [JsonPropertyName("PositionTicks")]
    public long? PositionTicks { get; set; }

    [JsonPropertyName("IsPaused")]
    public bool? IsPaused { get; set; }
}

// ===== Plex Webhook Payload =====

public class PlexWebhookPayload
{
    [JsonPropertyName("event")]
    public string? Event { get; set; }

    [JsonPropertyName("user")]
    public bool? User { get; set; }

    [JsonPropertyName("owner")]
    public bool? Owner { get; set; }

    [JsonPropertyName("Account")]
    public PlexAccount? Account { get; set; }

    [JsonPropertyName("Server")]
    public PlexServer? Server { get; set; }

    [JsonPropertyName("Player")]
    public PlexPlayer? Player { get; set; }

    [JsonPropertyName("Metadata")]
    public PlexMetadata? Metadata { get; set; }
}

public class PlexAccount
{
    [JsonPropertyName("id")]
    public int? Id { get; set; }

    [JsonPropertyName("title")]
    public string? Title { get; set; }
}

public class PlexServer
{
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    [JsonPropertyName("uuid")]
    public string? Uuid { get; set; }
}

public class PlexPlayer
{
    [JsonPropertyName("title")]
    public string? Title { get; set; }

    [JsonPropertyName("uuid")]
    public string? Uuid { get; set; }

    [JsonPropertyName("local")]
    public bool? Local { get; set; }
}

public class PlexMetadata
{
    [JsonPropertyName("ratingKey")]
    public string? RatingKey { get; set; }

    [JsonPropertyName("type")]
    public string? Type { get; set; }

    [JsonPropertyName("title")]
    public string? Title { get; set; }

    [JsonPropertyName("year")]
    public int? Year { get; set; }

    [JsonPropertyName("guid")]
    public string? GuidString { get; set; }

    // External GUIDs array (e.g., [{"id":"imdb://tt1234567"}, {"id":"tmdb://12345"}])
    [JsonPropertyName("Guid")]
    public PlexGuid[]? ExternalGuids { get; set; }

    [JsonPropertyName("viewOffset")]
    public long? ViewOffset { get; set; }

    [JsonPropertyName("duration")]
    public long? Duration { get; set; }

    [JsonPropertyName("index")]
    public int? Index { get; set; }

    [JsonPropertyName("parentIndex")]
    public int? ParentIndex { get; set; }
}

public class PlexGuid
{
    [JsonPropertyName("id")]
    public string? Id { get; set; }
}
