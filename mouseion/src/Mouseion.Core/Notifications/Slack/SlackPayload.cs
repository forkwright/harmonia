// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Mouseion.Core.Notifications.Slack
{
    /// <summary>
    /// Slack webhook payload structure
    /// See: https://api.slack.com/messaging/webhooks
    /// </summary>
    public class SlackPayload
    {
        [JsonPropertyName("text")]
        public string? Text { get; set; }

        [JsonPropertyName("username")]
        public string? Username { get; set; }

        [JsonPropertyName("icon_emoji")]
        public string? IconEmoji { get; set; }

        [JsonPropertyName("icon_url")]
        public string? IconUrl { get; set; }

        [JsonPropertyName("channel")]
        public string? Channel { get; set; }

        [JsonPropertyName("attachments")]
        public List<SlackAttachment>? Attachments { get; set; }
    }

    public class SlackAttachment
    {
        [JsonPropertyName("fallback")]
        public string? Fallback { get; set; }

        [JsonPropertyName("color")]
        public string? Color { get; set; }

        [JsonPropertyName("title")]
        public string? Title { get; set; }

        [JsonPropertyName("text")]
        public string? Text { get; set; }

        [JsonPropertyName("fields")]
        public List<SlackField>? Fields { get; set; }

        [JsonPropertyName("footer")]
        public string? Footer { get; set; }

        [JsonPropertyName("ts")]
        public long? Timestamp { get; set; }
    }

    public class SlackField
    {
        [JsonPropertyName("title")]
        public string Title { get; set; } = string.Empty;

        [JsonPropertyName("value")]
        public string Value { get; set; } = string.Empty;

        [JsonPropertyName("short")]
        public bool Short { get; set; }
    }

    /// <summary>
    /// Slack attachment colors
    /// </summary>
    public static class SlackColors
    {
        public const string Good = "good";      // Green
        public const string Warning = "warning"; // Orange
        public const string Danger = "danger";   // Red
        public const string Standard = "#3498db"; // Blue
        public const string Upgrade = "#9b59b6";  // Purple
    }
}
