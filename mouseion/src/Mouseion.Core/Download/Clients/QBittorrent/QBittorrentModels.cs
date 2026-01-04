// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.Download.Clients.QBittorrent;

public class QBittorrentTorrent
{
    [JsonPropertyName("hash")]
    public string Hash { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("size")]
    public long Size { get; set; }

    [JsonPropertyName("progress")]
    public double Progress { get; set; }

    [JsonPropertyName("eta")]
    public long Eta { get; set; }

    [JsonPropertyName("state")]
    public string State { get; set; } = string.Empty;

    [JsonPropertyName("label")]
    public string Label { get; set; } = string.Empty;

    [JsonPropertyName("category")]
    public string Category { get; set; } = string.Empty;

    [JsonPropertyName("save_path")]
    public string SavePath { get; set; } = string.Empty;

    [JsonPropertyName("content_path")]
    public string ContentPath { get; set; } = string.Empty;

    [JsonPropertyName("ratio")]
    public float Ratio { get; set; }

    [JsonPropertyName("ratio_limit")]
    public float RatioLimit { get; set; } = -2;

    [JsonPropertyName("seeding_time")]
    public long? SeedingTime { get; set; }

    [JsonPropertyName("seeding_time_limit")]
    public long SeedingTimeLimit { get; set; } = -2;

    [JsonPropertyName("last_activity")]
    public long LastActivity { get; set; }
}

public class QBittorrentPreferences
{
    [JsonPropertyName("save_path")]
    public string SavePath { get; set; } = string.Empty;

    [JsonPropertyName("max_ratio_enabled")]
    public bool MaxRatioEnabled { get; set; }

    [JsonPropertyName("max_ratio")]
    public float MaxRatio { get; set; }

    [JsonPropertyName("max_seeding_time_enabled")]
    public bool MaxSeedingTimeEnabled { get; set; }

    [JsonPropertyName("max_seeding_time")]
    public int MaxSeedingTime { get; set; }

    [JsonPropertyName("max_ratio_act")]
    public int MaxRatioAction { get; set; }

    [JsonPropertyName("dht")]
    public bool DhtEnabled { get; set; }

    [JsonPropertyName("queueing_enabled")]
    public bool QueueingEnabled { get; set; }
}

public class QBittorrentLabel
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("savePath")]
    public string SavePath { get; set; } = string.Empty;
}
