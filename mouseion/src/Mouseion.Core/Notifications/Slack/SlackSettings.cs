// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications.Slack
{
    /// <summary>
    /// Slack webhook notification settings
    /// </summary>
    public class SlackSettings : NotificationSettings
    {
        /// <summary>
        /// Slack webhook URL (from Workspace -> Apps -> Incoming Webhooks)
        /// </summary>
        public string WebhookUrl { get; set; } = string.Empty;

        /// <summary>
        /// Custom username to display in Slack (optional)
        /// </summary>
        public string? Username { get; set; }

        /// <summary>
        /// Custom icon emoji to display in Slack (optional, e.g., ":robot_face:")
        /// </summary>
        public string? IconEmoji { get; set; }

        /// <summary>
        /// Custom icon URL to display in Slack (optional, overrides IconEmoji)
        /// </summary>
        public string? IconUrl { get; set; }

        /// <summary>
        /// Channel to post to (optional, overrides webhook default)
        /// </summary>
        public string? Channel { get; set; }
    }
}
