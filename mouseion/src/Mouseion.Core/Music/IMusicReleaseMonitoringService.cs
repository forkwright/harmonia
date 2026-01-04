// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Music;

public interface IMusicReleaseMonitoringService
{
    Task<List<NewRelease>> CheckForNewReleasesAsync(int artistId, CancellationToken ct = default);
    Task<List<NewRelease>> CheckForNewReleasesAsync(string artistMbid, CancellationToken ct = default);
}

public class NewRelease
{
    public string Title { get; set; } = null!;
    public string ReleaseGroupMbid { get; set; } = null!;
    public string? ReleaseMbid { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? AlbumType { get; set; }
    public string? Country { get; set; }
    public List<string> Formats { get; set; } = new();
    public int? TrackCount { get; set; }
}
