// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications.Apprise
{
    /// <summary>
    /// Apprise notification settings (multi-service notification wrapper)
    /// </summary>
    public class AppriseSettings : NotificationSettings
    {
        /// <summary>
        /// Apprise API server URL (e.g., http://localhost:8000)
        /// </summary>
        public string ServerUrl { get; set; } = string.Empty;

        /// <summary>
        /// Configuration key (if using stateful mode)
        /// </summary>
        public string? ConfigurationKey { get; set; }

        /// <summary>
        /// Notification URLs (e.g., discord://webhook_id/webhook_token, slack://token/channel)
        /// Comma-separated for multiple services
        /// </summary>
        public string? NotificationUrls { get; set; }

        /// <summary>
        /// Tags to filter notifications (optional, comma-separated)
        /// </summary>
        public string? Tags { get; set; }
    }
}
