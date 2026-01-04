// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Mouseion.Core.Notifications.Discord
{
    /// <summary>
    /// Discord webhook payload structure
    /// See: https://discord.com/developers/docs/resources/webhook#execute-webhook
    /// </summary>
    public class DiscordPayload
    {
        [JsonPropertyName("content")]
        public string? Content { get; set; }

        [JsonPropertyName("username")]
        public string? Username { get; set; }

        [JsonPropertyName("avatar_url")]
        public string? AvatarUrl { get; set; }

        [JsonPropertyName("embeds")]
        public List<DiscordEmbed>? Embeds { get; set; }
    }

    public class DiscordEmbed
    {
        [JsonPropertyName("title")]
        public string? Title { get; set; }

        [JsonPropertyName("description")]
        public string? Description { get; set; }

        [JsonPropertyName("color")]
        public int? Color { get; set; }

        [JsonPropertyName("author")]
        public DiscordAuthor? Author { get; set; }

        [JsonPropertyName("fields")]
        public List<DiscordField>? Fields { get; set; }

        [JsonPropertyName("thumbnail")]
        public DiscordImage? Thumbnail { get; set; }

        [JsonPropertyName("image")]
        public DiscordImage? Image { get; set; }

        [JsonPropertyName("timestamp")]
        public string? Timestamp { get; set; }
    }

    public class DiscordAuthor
    {
        [JsonPropertyName("name")]
        public string? Name { get; set; }

        [JsonPropertyName("icon_url")]
        public string? IconUrl { get; set; }
    }

    public class DiscordField
    {
        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        [JsonPropertyName("value")]
        public string Value { get; set; } = string.Empty;

        [JsonPropertyName("inline")]
        public bool Inline { get; set; }
    }

    public class DiscordImage
    {
        [JsonPropertyName("url")]
        public string? Url { get; set; }
    }

    /// <summary>
    /// Discord embed colors
    /// </summary>
    public static class DiscordColors
    {
        public const int Standard = 0x3498db; // Blue
        public const int Success = 0x2ecc71;  // Green
        public const int Warning = 0xf39c12;  // Orange
        public const int Danger = 0xe74c3c;   // Red
        public const int Upgrade = 0x9b59b6;  // Purple
    }
}
