// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Indexers.Torznab;

public class TorznabRelease
{
    public string Title { get; set; } = string.Empty;
    public long Size { get; set; }
    public DateTime? PublishDate { get; set; }
    public string? DownloadUrl { get; set; }
    public string? InfoUrl { get; set; }
    public int Seeders { get; set; }
    public int Peers { get; set; }
}
