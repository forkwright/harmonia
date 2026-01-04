// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download;

public class DownloadClientInfo
{
    public bool IsLocalhost { get; set; }
    public string SortingMode { get; set; } = string.Empty;
    public bool RemovesCompletedDownloads { get; set; }
    public List<string> OutputRootFolders { get; set; } = new();
}
