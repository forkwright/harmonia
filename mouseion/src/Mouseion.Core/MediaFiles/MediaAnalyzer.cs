// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using TagLib;

namespace Mouseion.Core.MediaFiles;

public interface IMediaAnalyzer
{
    List<ChapterInfo> GetChapters(string filePath);
    MediaFileInfo GetMediaInfo(string filePath);
}

public class MediaAnalyzer : IMediaAnalyzer
{
    public List<ChapterInfo> GetChapters(string filePath)
    {
        var chapters = new List<ChapterInfo>();

        try
        {
            using var file = TagLib.File.Create(filePath);

            // TODO: Implement M4B chpl atom parsing and MP3 ID3v2 CHAP frame parsing
            // TagLibSharp doesn't expose these directly - need custom parser
            // For now, return single chapter spanning the file

            if (file.Properties.Duration.TotalMilliseconds > 0)
            {
                chapters.Add(new ChapterInfo
                {
                    Index = 0,
                    Title = "Full Book",
                    StartTimeMs = 0,
                    EndTimeMs = (long)file.Properties.Duration.TotalMilliseconds
                });
            }
        }
        catch
        {
            // Graceful degradation: return empty list if file can't be parsed
        }

        return chapters;
    }

    public MediaFileInfo GetMediaInfo(string filePath)
    {
        using var file = TagLib.File.Create(filePath);

        return new MediaFileInfo
        {
            DurationSeconds = (int)file.Properties.Duration.TotalSeconds,
            Bitrate = file.Properties.AudioBitrate,
            SampleRate = file.Properties.AudioSampleRate,
            Channels = file.Properties.AudioChannels,
            Format = file.MimeType
        };
    }
}

public class MediaFileInfo
{
    public int DurationSeconds { get; set; }
    public int Bitrate { get; set; }
    public int SampleRate { get; set; }
    public int Channels { get; set; }
    public string Format { get; set; } = string.Empty;
}
