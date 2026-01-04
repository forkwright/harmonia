// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download;

public enum DownloadItemStatus
{
    Queued = 0,
    Paused = 1,
    Downloading = 2,
    Completed = 3,
    Failed = 4,
    Warning = 5
}
