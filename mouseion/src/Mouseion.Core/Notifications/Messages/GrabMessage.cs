// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when media is grabbed/queued for download
    /// </summary>
    public class GrabMessage
    {
        public string Title { get; set; } = string.Empty;
        public string MediaType { get; set; } = string.Empty; // Movie, Book, Audiobook, Music
        public string Quality { get; set; } = string.Empty;
        public long SizeBytes { get; set; }
        public string ReleaseGroup { get; set; } = string.Empty;
        public string Indexer { get; set; } = string.Empty;
        public string DownloadClient { get; set; } = string.Empty;
    }
}
