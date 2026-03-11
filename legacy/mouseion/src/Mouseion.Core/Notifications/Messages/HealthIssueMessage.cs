// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when health check fails
    /// </summary>
    public class HealthIssueMessage
    {
        public string Source { get; set; } = string.Empty;
        public string Type { get; set; } = string.Empty; // Warning, Error
        public string Message { get; set; } = string.Empty;
        public string WikiUrl { get; set; } = string.Empty;
    }
}
