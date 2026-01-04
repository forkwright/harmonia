// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when media is deleted from library
    /// </summary>
    public class MediaDeletedMessage
    {
        public string Title { get; set; } = string.Empty;
        public string MediaType { get; set; } = string.Empty;
        public bool DeletedFiles { get; set; }
        public string Reason { get; set; } = string.Empty;
    }
}
