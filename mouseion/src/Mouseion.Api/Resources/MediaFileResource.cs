// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Api.Resources;

/// <summary>
/// Shared media file resource used by MediaFiles and MediaItems controllers.
/// </summary>
public class MediaFileResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string MediaType { get; set; } = null!;
    public string Path { get; set; } = null!;
    public string? RelativePath { get; set; }
    public long Size { get; set; }
    public DateTime DateAdded { get; set; }
    public int? DurationSeconds { get; set; }
    public int? Bitrate { get; set; }
    public int? SampleRate { get; set; }
    public int? Channels { get; set; }
    public string? Format { get; set; }
    public string? Quality { get; set; }
    public string? FileHash { get; set; }
}
