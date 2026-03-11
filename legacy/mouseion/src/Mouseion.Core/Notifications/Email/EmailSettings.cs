// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications.Email
{
    /// <summary>
    /// Email notification settings (SMTP)
    /// </summary>
    public class EmailSettings : NotificationSettings
    {
        /// <summary>
        /// SMTP server hostname
        /// </summary>
        public string Server { get; set; } = string.Empty;

        /// <summary>
        /// SMTP port (25, 587, or 465)
        /// </summary>
        public int Port { get; set; } = 587;

        /// <summary>
        /// Use SSL/TLS encryption
        /// </summary>
        public bool UseSsl { get; set; } = true;

        /// <summary>
        /// SMTP username (if authentication required)
        /// </summary>
        public string? Username { get; set; }

        /// <summary>
        /// SMTP password (if authentication required)
        /// </summary>
        public string? Password { get; set; }

        /// <summary>
        /// From email address
        /// </summary>
        public string FromAddress { get; set; } = string.Empty;

        /// <summary>
        /// From display name (optional)
        /// </summary>
        public string? FromName { get; set; }

        /// <summary>
        /// To email addresses (comma-separated)
        /// </summary>
        public string ToAddresses { get; set; } = string.Empty;

        /// <summary>
        /// CC email addresses (comma-separated, optional)
        /// </summary>
        public string? CcAddresses { get; set; }

        /// <summary>
        /// BCC email addresses (comma-separated, optional)
        /// </summary>
        public string? BccAddresses { get; set; }
    }
}
