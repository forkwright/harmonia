// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Download.DelayProfiles;

/// <summary>
/// Quality-conscious acquisition delay — wait for better releases before grabbing.
/// When a release meets minimum quality but not preferred quality, hold for the delay period.
/// If preferred quality arrives during the delay, grab immediately.
/// </summary>
public class DelayProfile : ModelBase
{
    /// <summary>
    /// Media type this profile applies to. Null = all media types.
    /// </summary>
    public MediaType? MediaType { get; set; }

    /// <summary>
    /// Name for organizational purposes.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Preferred protocol. Usenet is typically preferred (faster, more complete) but torrent
    /// may be preferred for seeding ratios.
    /// </summary>
    public DownloadProtocol PreferredProtocol { get; set; } = DownloadProtocol.Usenet;

    /// <summary>
    /// Delay in hours for Usenet releases that don't meet preferred quality.
    /// 0 = no delay, grab immediately.
    /// </summary>
    public int UsenetDelay { get; set; }

    /// <summary>
    /// Delay in hours for Torrent releases that don't meet preferred quality.
    /// 0 = no delay, grab immediately.
    /// </summary>
    public int TorrentDelay { get; set; }

    /// <summary>
    /// Quality weight threshold. Releases at or above this weight bypass the delay entirely.
    /// References QualityDefinition.Weight from the quality system.
    /// </summary>
    public int PreferredQualityWeight { get; set; }

    /// <summary>
    /// If true, releases that meet or exceed PreferredQualityWeight are grabbed immediately
    /// regardless of any delay setting.
    /// </summary>
    public bool BypassIfPreferredQuality { get; set; } = true;

    /// <summary>
    /// If true, releases with preferred word matches (from quality profiles) bypass the delay.
    /// </summary>
    public bool BypassIfPreferredWords { get; set; }

    /// <summary>
    /// Comma-separated tag IDs for scoping. Different delays for different libraries
    /// (e.g., 4K movies wait 7 days, 1080p grabs immediately).
    /// Empty = applies to all untagged items.
    /// </summary>
    public string? Tags { get; set; }

    /// <summary>
    /// Priority order. Lower = higher priority. When multiple profiles match, lowest order wins.
    /// </summary>
    public int Order { get; set; }

    /// <summary>
    /// Whether this profile is active.
    /// </summary>
    public bool Enabled { get; set; } = true;
}
