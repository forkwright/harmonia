// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when media is added to library
    /// </summary>
    public class MediaAddedMessage
    {
        public string Title { get; set; } = string.Empty;
        public string MediaType { get; set; } = string.Empty;
        public int Year { get; set; }
        public string Overview { get; set; } = string.Empty;
    }
}
