// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when media download completes and is imported
    /// </summary>
    public class DownloadMessage
    {
        public string Title { get; set; } = string.Empty;
        public string MediaType { get; set; } = string.Empty; // Movie, Book, Audiobook, Music
        public string Quality { get; set; } = string.Empty;
        public long SizeBytes { get; set; }
        public string FilePath { get; set; } = string.Empty;
        public bool IsUpgrade { get; set; }
    }
}
