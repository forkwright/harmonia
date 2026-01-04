// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Blocklisting;

public class Blocklist : ModelBase
{
    public int MediaItemId { get; set; }
    public string SourceTitle { get; set; } = null!;
    public QualityModel Quality { get; set; } = null!;
    public DateTime Date { get; set; }
    public DateTime? PublishedDate { get; set; }
    public long? Size { get; set; }
    public DownloadProtocol Protocol { get; set; }
    public string? Indexer { get; set; }
    public string? Message { get; set; }
    public string? TorrentInfoHash { get; set; }
}

public enum DownloadProtocol
{
    Unknown = 0,
    Usenet = 1,
    Torrent = 2
}
