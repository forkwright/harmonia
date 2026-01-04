// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Base class for notification provider settings
    /// </summary>
    public abstract class NotificationSettings
    {
        /// <summary>
        /// Unique ID for this notification configuration
        /// </summary>
        public int Id { get; set; }

        /// <summary>
        /// User-friendly name for this notification
        /// </summary>
        public string Name { get; set; } = string.Empty;

        /// <summary>
        /// Whether this notification is enabled
        /// </summary>
        public bool Enabled { get; set; }

        /// <summary>
        /// Trigger on media grab/queue
        /// </summary>
        public bool OnGrab { get; set; }

        /// <summary>
        /// Trigger on successful download/import
        /// </summary>
        public bool OnDownload { get; set; }

        /// <summary>
        /// Trigger on media file rename
        /// </summary>
        public bool OnRename { get; set; }

        /// <summary>
        /// Trigger when media added to library
        /// </summary>
        public bool OnMediaAdded { get; set; }

        /// <summary>
        /// Trigger when media deleted from library
        /// </summary>
        public bool OnMediaDeleted { get; set; }

        /// <summary>
        /// Trigger on health check failure
        /// </summary>
        public bool OnHealthIssue { get; set; }

        /// <summary>
        /// Trigger when health check restored
        /// </summary>
        public bool OnHealthRestored { get; set; }

        /// <summary>
        /// Trigger on application update
        /// </summary>
        public bool OnApplicationUpdate { get; set; }
    }
}
