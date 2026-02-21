// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Api.SmartPlaylists;

public class SmartPlaylistResource
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("filterRequestJson")]
    public string FilterRequestJson { get; set; } = "{}";

    [JsonPropertyName("trackCount")]
    public int TrackCount { get; set; }

    [JsonPropertyName("lastRefreshed")]
    public DateTime LastRefreshed { get; set; }

    [JsonPropertyName("createdAt")]
    public DateTime CreatedAt { get; set; }

    [JsonPropertyName("updatedAt")]
    public DateTime UpdatedAt { get; set; }

    [JsonPropertyName("tracks")]
    public List<SmartPlaylistTrackResource>? Tracks { get; set; }
}

public class SmartPlaylistTrackResource
{
    [JsonPropertyName("trackId")]
    public int TrackId { get; set; }

    [JsonPropertyName("position")]
    public int Position { get; set; }
}
