// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Api.Notifications
{
    /// <summary>
    /// API resource for notification configuration
    /// </summary>
    public class NotificationResource
    {
        [JsonPropertyName("id")]
        public int Id { get; set; }

        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        [JsonPropertyName("type")]
        public string Type { get; set; } = string.Empty; // Discord, Slack, Telegram, etc.

        [JsonPropertyName("enabled")]
        public bool Enabled { get; set; }

        [JsonPropertyName("onGrab")]
        public bool OnGrab { get; set; }

        [JsonPropertyName("onDownload")]
        public bool OnDownload { get; set; }

        [JsonPropertyName("onRename")]
        public bool OnRename { get; set; }

        [JsonPropertyName("onMediaAdded")]
        public bool OnMediaAdded { get; set; }

        [JsonPropertyName("onMediaDeleted")]
        public bool OnMediaDeleted { get; set; }

        [JsonPropertyName("onHealthIssue")]
        public bool OnHealthIssue { get; set; }

        [JsonPropertyName("onHealthRestored")]
        public bool OnHealthRestored { get; set; }

        [JsonPropertyName("onApplicationUpdate")]
        public bool OnApplicationUpdate { get; set; }

        [JsonPropertyName("settings")]
        public object Settings { get; set; } = new();
    }
}
