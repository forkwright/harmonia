// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when health check is restored
    /// </summary>
    public class HealthRestoredMessage
    {
        public string Source { get; set; } = string.Empty;
        public string PreviousMessage { get; set; } = string.Empty;
    }
}
