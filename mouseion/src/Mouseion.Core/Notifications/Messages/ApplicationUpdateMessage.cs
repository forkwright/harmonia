// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when application is updated
    /// </summary>
    public class ApplicationUpdateMessage
    {
        public string PreviousVersion { get; set; } = string.Empty;
        public string NewVersion { get; set; } = string.Empty;
        public string ReleaseNotes { get; set; } = string.Empty;
    }
}
