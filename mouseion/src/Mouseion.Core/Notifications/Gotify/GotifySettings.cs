// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications.Gotify
{
    /// <summary>
    /// Gotify notification settings (self-hosted push notification service)
    /// </summary>
    public class GotifySettings : NotificationSettings
    {
        /// <summary>
        /// Gotify server URL (e.g., http://localhost:8080)
        /// </summary>
        public string ServerUrl { get; set; } = string.Empty;

        /// <summary>
        /// Application token (from Gotify Apps page)
        /// </summary>
        public string AppToken { get; set; } = string.Empty;

        /// <summary>
        /// Priority (1=low, 5=normal, 10=high)
        /// </summary>
        public int Priority { get; set; } = 5;
    }
}
