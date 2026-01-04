// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Notification message when media file is renamed
    /// </summary>
    public class RenameMessage
    {
        public string Title { get; set; } = string.Empty;
        public string MediaType { get; set; } = string.Empty;
        public string OldPath { get; set; } = string.Empty;
        public string NewPath { get; set; } = string.Empty;
    }
}
