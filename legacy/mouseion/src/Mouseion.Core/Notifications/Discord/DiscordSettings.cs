// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications.Discord
{
    /// <summary>
    /// Discord webhook notification settings
    /// </summary>
    public class DiscordSettings : NotificationSettings
    {
        /// <summary>
        /// Discord webhook URL (from Server Settings -> Integrations -> Webhooks)
        /// </summary>
        public string WebhookUrl { get; set; } = string.Empty;

        /// <summary>
        /// Custom username to display in Discord (optional, overrides webhook default)
        /// </summary>
        public string? Username { get; set; }

        /// <summary>
        /// Custom avatar URL to display in Discord (optional)
        /// </summary>
        public string? AvatarUrl { get; set; }

        /// <summary>
        /// Author name to display in embed (optional, defaults to "Mouseion")
        /// </summary>
        public string? Author { get; set; }
    }
}
