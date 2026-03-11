// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.MediaFiles.MediaInfo;

public class MediaInfoModel
{
    public string? RawStreamData { get; set; }
    public string? RawFrameData { get; set; }
    public int SchemaRevision { get; set; }

    public string? ContainerFormat { get; set; }
    public string? VideoFormat { get; set; }
    public string? VideoCodecID { get; set; }
    public string? VideoProfile { get; set; }
    public long VideoBitrate { get; set; }
    public int VideoBitDepth { get; set; }
    public int VideoMultiViewCount { get; set; }
    public string? VideoColourPrimaries { get; set; }
    public string? VideoTransferCharacteristics { get; set; }
    public HdrFormat VideoHdrFormat { get; set; }
    public int Height { get; set; }
    public int Width { get; set; }

    public string? AudioFormat { get; set; }
    public string? AudioCodecID { get; set; }
    public string? AudioProfile { get; set; }
    public long AudioBitrate { get; set; }
    public TimeSpan RunTime { get; set; }
    public int AudioStreamCount { get; set; }
    public int AudioChannels { get; set; }
    public string? AudioChannelPositions { get; set; }
    public decimal VideoFps { get; set; }
    public List<string>? AudioLanguages { get; set; }
    public List<string>? Subtitles { get; set; }
    public string? ScanType { get; set; }

    [JsonIgnore]
    public string? Title { get; set; }
}
