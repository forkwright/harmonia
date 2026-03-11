// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Serilog;

namespace Mouseion.Core.MediaFiles.MediaInfo;

public class MediaInfoService : IMediaInfoService
{
    private readonly ILogger<MediaInfoService> _logger;
    private readonly string _ffprobePath;
    private static readonly Serilog.ILogger Logger = Log.ForContext<MediaInfoService>();

    public const int MINIMUM_MEDIA_INFO_SCHEMA_REVISION = 14;
    public const int CURRENT_MEDIA_INFO_SCHEMA_REVISION = 14;

    private static readonly string[] ValidHdrColourPrimaries = { "bt2020" };
    private static readonly string[] HlgTransferFunctions = { "arib-std-b67" };
    private static readonly string[] PqTransferFunctions = { "smpte2084" };
    private static readonly string[] ValidHdrTransferFunctions = HlgTransferFunctions.Concat(PqTransferFunctions).ToArray();

    public MediaInfoService(ILogger<MediaInfoService> logger)
    {
        _logger = logger;
        _ffprobePath = FindFfprobe();
    }

    public MediaInfoModel? GetMediaInfo(string filename)
    {
        if (!File.Exists(filename))
        {
            throw new FileNotFoundException("Media file does not exist: " + filename);
        }

        if (MediaFileExtensions.DiskExtensions.Contains(Path.GetExtension(filename)))
        {
            return null;
        }

        try
        {
            _logger.LogDebug("Getting media info from {Filename}", SanitizeForLog(filename));

            var streamJson = GetStreamJson(filename, "-probesize 50000000");
            var analysis = ParseStreamJson(streamJson);
            var primaryVideoStream = GetPrimaryVideoStream(analysis);

            if (string.IsNullOrWhiteSpace(analysis.PrimaryAudioStream?.ChannelLayout))
            {
                streamJson = GetStreamJson(filename, "-probesize 150000000 -analyzeduration 150000000");
                analysis = ParseStreamJson(streamJson);
            }

            var mediaInfoModel = new MediaInfoModel
            {
                ContainerFormat = analysis.Format?.FormatName,
                VideoFormat = primaryVideoStream?.CodecName,
                VideoCodecID = primaryVideoStream?.CodecTagString,
                VideoProfile = primaryVideoStream?.Profile,
                VideoBitrate = GetBitrate(primaryVideoStream),
                VideoMultiViewCount = primaryVideoStream?.Tags?.ContainsKey("stereo_mode") ?? false ? 2 : 1,
                VideoBitDepth = GetVideoBitDepth(primaryVideoStream?.PixelFormat) ?? 8,
                VideoColourPrimaries = primaryVideoStream?.ColorPrimaries,
                VideoTransferCharacteristics = primaryVideoStream?.ColorTransfer,
                Height = primaryVideoStream?.Height ?? 0,
                Width = primaryVideoStream?.Width ?? 0,
                AudioFormat = analysis.PrimaryAudioStream?.CodecName,
                AudioCodecID = analysis.PrimaryAudioStream?.CodecTagString,
                AudioProfile = analysis.PrimaryAudioStream?.Profile,
                AudioBitrate = GetBitrate(analysis.PrimaryAudioStream),
                RunTime = GetBestRuntime(analysis.PrimaryAudioStream?.Duration, primaryVideoStream?.Duration, analysis.Format?.Duration ?? TimeSpan.Zero),
                AudioStreamCount = analysis.AudioStreams?.Count ?? 0,
                AudioChannels = analysis.PrimaryAudioStream?.Channels ?? 0,
                AudioChannelPositions = analysis.PrimaryAudioStream?.ChannelLayout,
                VideoFps = primaryVideoStream?.FrameRate ?? 0,
                AudioLanguages = analysis.AudioStreams?
                    .Select(x => x.Language)
                    .Where(l => !string.IsNullOrWhiteSpace(l))
                    .ToList(),
                Subtitles = analysis.SubtitleStreams?
                    .Select(x => x.Language)
                    .Where(l => !string.IsNullOrWhiteSpace(l))
                    .ToList(),
                ScanType = "Progressive",
                RawStreamData = streamJson,
                SchemaRevision = CURRENT_MEDIA_INFO_SCHEMA_REVISION
            };

            if (analysis.Format?.Tags?.TryGetValue("title", out var title) ?? false)
            {
                mediaInfoModel.Title = title;
            }

            var sideData = primaryVideoStream?.SideDataList ?? new List<string>();

            if (PqTransferFunctions.Contains(mediaInfoModel.VideoTransferCharacteristics))
            {
                var frameJson = GetFrameJson(filename, primaryVideoStream?.Index ?? 0);
                mediaInfoModel.RawFrameData = frameJson;
                var frameSideData = ParseFrameSideData(frameJson);
                sideData = sideData.Concat(frameSideData).ToList();
            }

            mediaInfoModel.VideoHdrFormat = GetHdrFormat(
                mediaInfoModel.VideoBitDepth,
                mediaInfoModel.VideoColourPrimaries,
                mediaInfoModel.VideoTransferCharacteristics,
                sideData);

            return mediaInfoModel;
        }
        catch (System.ComponentModel.Win32Exception ex)
        {
            _logger.LogError(ex, "Failed to start ffprobe process for file: {Filename}", SanitizeForLog(filename));
        }
        catch (IOException ex)
        {
            _logger.LogError(ex, "I/O error parsing media info from file: {Filename}", SanitizeForLog(filename));
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "JSON parsing error for media info from file: {Filename}", SanitizeForLog(filename));
        }
        catch (InvalidOperationException ex)
        {
            _logger.LogError(ex, "Invalid operation parsing media info from file: {Filename}", SanitizeForLog(filename));
        }

        return null;
    }

    public TimeSpan? GetRunTime(string filename)
    {
        var info = GetMediaInfo(filename);
        return info?.RunTime;
    }

    private string GetStreamJson(string filename, string extraArgs)
    {
        var args = $"-v quiet -print_format json -show_format -show_streams {extraArgs} \"{filename}\"";
        return RunFfprobe(args);
    }

    private string GetFrameJson(string filename, int videoStreamIndex)
    {
        var args = $"-v quiet -print_format json -show_frames -read_intervals \"%+#1\" -select_streams v:{videoStreamIndex} \"{filename}\"";
        return RunFfprobe(args);
    }

    private string RunFfprobe(string arguments)
    {
        if (string.IsNullOrEmpty(_ffprobePath))
        {
            throw new InvalidOperationException("ffprobe not found. Please install ffmpeg.");
        }

        var psi = new ProcessStartInfo
        {
            FileName = _ffprobePath,
            Arguments = arguments,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true
        };

        using var process = Process.Start(psi);
        if (process == null)
        {
            throw new InvalidOperationException("Failed to start ffprobe process");
        }

        var output = process.StandardOutput.ReadToEnd();
        var error = process.StandardError.ReadToEnd();
        process.WaitForExit();

        if (process.ExitCode != 0)
        {
            throw new InvalidOperationException($"ffprobe failed with exit code {process.ExitCode}: {error}");
        }

        return output;
    }

    private static string FindFfprobe()
    {
        var paths = new[]
        {
            "ffprobe",
            "/usr/bin/ffprobe",
            "/usr/local/bin/ffprobe",
            Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "ffprobe"),
            Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "ffprobe.exe")
        };

        foreach (var path in paths)
        {
            try
            {
                var psi = new ProcessStartInfo
                {
                    FileName = path,
                    Arguments = "-version",
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    CreateNoWindow = true
                };

                using var process = Process.Start(psi);
                if (process != null)
                {
                    process.WaitForExit();
                    if (process.ExitCode == 0)
                    {
                        return path;
                    }
                }
            }
            catch (Exception ex)
            {
                Logger.Debug(ex, "Failed to execute ffprobe at path: {Path}", path);
            }
        }

        return string.Empty;
    }

    private FFProbeAnalysis ParseStreamJson(string json)
    {
        var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        var analysis = new FFProbeAnalysis
        {
            Format = ParseFormat(root.GetProperty("format")),
            VideoStreams = new List<VideoStream>(),
            AudioStreams = new List<AudioStream>(),
            SubtitleStreams = new List<SubtitleStream>()
        };

        if (root.TryGetProperty("streams", out var streams))
        {
            foreach (var stream in streams.EnumerateArray())
            {
                var codecType = stream.GetProperty("codec_type").GetString();

                if (codecType == "video")
                {
                    analysis.VideoStreams.Add(ParseVideoStream(stream));
                }
                else if (codecType == "audio")
                {
                    analysis.AudioStreams.Add(ParseAudioStream(stream));
                }
                else if (codecType == "subtitle")
                {
                    analysis.SubtitleStreams.Add(ParseSubtitleStream(stream));
                }
            }
        }

        analysis.PrimaryVideoStream = analysis.VideoStreams.FirstOrDefault();
        analysis.PrimaryAudioStream = analysis.AudioStreams.FirstOrDefault();

        return analysis;
    }

    private FormatInfo ParseFormat(JsonElement format)
    {
        var info = new FormatInfo
        {
            FormatName = format.GetPropertyOrNull("format_name")?.GetString(),
            Duration = ParseDuration(format.GetPropertyOrNull("duration")?.GetString()),
            Tags = new Dictionary<string, string>()
        };

        if (format.TryGetProperty("tags", out var tags))
        {
            foreach (var tag in tags.EnumerateObject())
            {
                info.Tags[tag.Name] = tag.Value.GetString() ?? string.Empty;
            }
        }

        return info;
    }

    private VideoStream ParseVideoStream(JsonElement stream)
    {
        return new VideoStream
        {
            Index = stream.GetPropertyOrNull("index")?.GetInt32() ?? 0,
            CodecName = stream.GetPropertyOrNull("codec_name")?.GetString(),
            CodecTagString = stream.GetPropertyOrNull("codec_tag_string")?.GetString(),
            Profile = stream.GetPropertyOrNull("profile")?.GetString(),
            Width = stream.GetPropertyOrNull("width")?.GetInt32() ?? 0,
            Height = stream.GetPropertyOrNull("height")?.GetInt32() ?? 0,
            PixelFormat = stream.GetPropertyOrNull("pix_fmt")?.GetString(),
            ColorPrimaries = stream.GetPropertyOrNull("color_primaries")?.GetString(),
            ColorTransfer = stream.GetPropertyOrNull("color_transfer")?.GetString(),
            Duration = ParseDuration(stream.GetPropertyOrNull("duration")?.GetString()),
            BitRate = ParseBitrate(stream.GetPropertyOrNull("bit_rate")?.GetString()),
            FrameRate = ParseFrameRate(stream.GetPropertyOrNull("r_frame_rate")?.GetString()),
            SideDataList = ParseSideDataList(stream),
            Tags = ParseTags(stream)
        };
    }

    private AudioStream ParseAudioStream(JsonElement stream)
    {
        return new AudioStream
        {
            CodecName = stream.GetPropertyOrNull("codec_name")?.GetString(),
            CodecTagString = stream.GetPropertyOrNull("codec_tag_string")?.GetString(),
            Profile = stream.GetPropertyOrNull("profile")?.GetString(),
            Channels = stream.GetPropertyOrNull("channels")?.GetInt32() ?? 0,
            ChannelLayout = stream.GetPropertyOrNull("channel_layout")?.GetString(),
            Language = stream.GetPropertyOrNull("tags")?.GetPropertyOrNull("language")?.GetString() ?? string.Empty,
            Duration = ParseDuration(stream.GetPropertyOrNull("duration")?.GetString()),
            BitRate = ParseBitrate(stream.GetPropertyOrNull("bit_rate")?.GetString()),
            Tags = ParseTags(stream)
        };
    }

    private SubtitleStream ParseSubtitleStream(JsonElement stream)
    {
        return new SubtitleStream
        {
            Language = stream.GetPropertyOrNull("tags")?.GetPropertyOrNull("language")?.GetString() ?? string.Empty
        };
    }

    private Dictionary<string, string> ParseTags(JsonElement stream)
    {
        var tags = new Dictionary<string, string>();

        if (stream.TryGetProperty("tags", out var tagsElement))
        {
            foreach (var tag in tagsElement.EnumerateObject())
            {
                tags[tag.Name] = tag.Value.GetString() ?? string.Empty;
            }
        }

        return tags;
    }

    private List<string> ParseSideDataList(JsonElement stream)
    {
        var sideData = new List<string>();

        if (stream.TryGetProperty("side_data_list", out var sideDataList))
        {
            foreach (var data in sideDataList.EnumerateArray())
            {
                if (data.TryGetProperty("side_data_type", out var type))
                {
                    sideData.Add(type.GetString() ?? string.Empty);
                }
            }
        }

        return sideData;
    }

    private List<string> ParseFrameSideData(string json)
    {
        var sideData = new List<string>();

        try
        {
            var doc = JsonDocument.Parse(json);
            if (doc.RootElement.TryGetProperty("frames", out var frames))
            {
                // Only process first frame
                var firstFrame = frames.EnumerateArray().FirstOrDefault();
                if (firstFrame.ValueKind != default && firstFrame.TryGetProperty("side_data_list", out var sideDataList))
                {
                    foreach (var data in sideDataList.EnumerateArray())
                    {
                        if (data.TryGetProperty("side_data_type", out var type))
                        {
                            sideData.Add(type.GetString() ?? string.Empty);
                        }
                    }
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to parse frame side data from JSON");
        }

        return sideData;
    }

    private static TimeSpan ParseDuration(string? duration)
    {
        if (string.IsNullOrEmpty(duration) || !double.TryParse(duration, out var seconds))
        {
            return TimeSpan.Zero;
        }

        return TimeSpan.FromSeconds(seconds);
    }

    private static long ParseBitrate(string? bitrate)
    {
        if (string.IsNullOrEmpty(bitrate) || !long.TryParse(bitrate, out var value))
        {
            return 0;
        }

        return value;
    }

    private static decimal ParseFrameRate(string? frameRate)
    {
        if (string.IsNullOrEmpty(frameRate))
        {
            return 0;
        }

        var parts = frameRate.Split('/');
        if (parts.Length == 2 &&
            decimal.TryParse(parts[0], out var numerator) &&
            decimal.TryParse(parts[1], out var denominator) &&
            denominator > 0)
        {
            return numerator / denominator;
        }

        return 0;
    }

    private static TimeSpan GetBestRuntime(TimeSpan? audio, TimeSpan? video, TimeSpan general)
    {
        if ((!video.HasValue || video.Value == TimeSpan.Zero) && (!audio.HasValue || audio.Value == TimeSpan.Zero))
        {
            return general;
        }

        if (!video.HasValue || video.Value == TimeSpan.Zero)
        {
            return audio!.Value;
        }

        return video.Value;
    }

    private static long GetBitrate(VideoStream? videoStream)
    {
        if (videoStream?.BitRate is > 0)
        {
            return videoStream.BitRate;
        }

        if ((videoStream?.Tags?.TryGetValue("BPS", out var bitratePerSecond) ?? false) && !string.IsNullOrWhiteSpace(bitratePerSecond))
        {
            return Convert.ToInt64(bitratePerSecond);
        }

        return 0;
    }

    private static long GetBitrate(AudioStream? audioStream)
    {
        if (audioStream?.BitRate is > 0)
        {
            return audioStream.BitRate;
        }

        if ((audioStream?.Tags?.TryGetValue("BPS", out var bitratePerSecond) ?? false) && !string.IsNullOrWhiteSpace(bitratePerSecond))
        {
            return Convert.ToInt64(bitratePerSecond);
        }

        return 0;
    }

    private static VideoStream? GetPrimaryVideoStream(FFProbeAnalysis mediaAnalysis)
    {
        if (mediaAnalysis.VideoStreams == null || mediaAnalysis.VideoStreams.Count <= 1)
        {
            return mediaAnalysis.PrimaryVideoStream;
        }

        var codecFilter = new[] { "mjpeg", "png" };
        return mediaAnalysis.VideoStreams.FirstOrDefault(s => !codecFilter.Contains(s.CodecName)) ?? mediaAnalysis.PrimaryVideoStream;
    }

    private static int? GetVideoBitDepth(string? pixelFormat)
    {
        if (string.IsNullOrEmpty(pixelFormat))
        {
            return 8;
        }

        if (pixelFormat.Contains("10le") || pixelFormat.Contains("10be") || pixelFormat.Contains("p010"))
        {
            return 10;
        }

        if (pixelFormat.Contains("12le") || pixelFormat.Contains("12be"))
        {
            return 12;
        }

        return 8;
    }

    private static HdrFormat GetHdrFormat(int bitDepth, string? colorPrimaries, string? transferFunction, List<string> sideData)
    {
        if (bitDepth < 10)
        {
            return HdrFormat.None;
        }

        var hasDovi = sideData.Any(x => x.Contains("DOVI") || x.Contains("Dolby"));
        var hasHdr10Plus = sideData.Any(x => x.Contains("HDR10+") || x.Contains("Dynamic Metadata SMPTE2094"));
        var hasMasteringMetadata = sideData.Any(x => x.Contains("Mastering") || x.Contains("Content light level"));

        if (hasDovi)
        {
            return GetDolbyVisionFormat(transferFunction, hasHdr10Plus);
        }

        if (!ValidHdrColourPrimaries.Contains(colorPrimaries) || !ValidHdrTransferFunctions.Contains(transferFunction))
        {
            return HdrFormat.None;
        }

        return GetStandardHdrFormat(transferFunction, hasHdr10Plus, hasMasteringMetadata);
    }

    private static HdrFormat GetDolbyVisionFormat(string? transferFunction, bool hasHdr10Plus)
    {
        if (hasHdr10Plus)
        {
            return HdrFormat.DolbyVisionHdr10Plus;
        }

        if (HlgTransferFunctions.Contains(transferFunction))
        {
            return HdrFormat.DolbyVisionHlg;
        }

        if (PqTransferFunctions.Contains(transferFunction))
        {
            return HdrFormat.DolbyVisionHdr10;
        }

        return HdrFormat.DolbyVision;
    }

    private static HdrFormat GetStandardHdrFormat(string? transferFunction, bool hasHdr10Plus, bool hasMasteringMetadata)
    {
        if (HlgTransferFunctions.Contains(transferFunction))
        {
            return HdrFormat.Hlg10;
        }

        if (PqTransferFunctions.Contains(transferFunction))
        {
            if (hasHdr10Plus)
            {
                return HdrFormat.Hdr10Plus;
            }

            if (hasMasteringMetadata)
            {
                return HdrFormat.Hdr10;
            }

            return HdrFormat.Pq10;
        }

        return HdrFormat.None;
    }

    private static string SanitizeForLog(string input)
    {
        return input.Replace("\r", "").Replace("\n", "");
    }

    private sealed class FFProbeAnalysis
    {
        public FormatInfo? Format { get; set; }
        public List<VideoStream>? VideoStreams { get; set; }
        public List<AudioStream>? AudioStreams { get; set; }
        public List<SubtitleStream>? SubtitleStreams { get; set; }
        public VideoStream? PrimaryVideoStream { get; set; }
        public AudioStream? PrimaryAudioStream { get; set; }
    }

    private sealed class FormatInfo
    {
        public string? FormatName { get; set; }
        public TimeSpan Duration { get; set; }
        public Dictionary<string, string> Tags { get; set; } = new();
    }

    private sealed class VideoStream
    {
        public int Index { get; set; }
        public string? CodecName { get; set; }
        public string? CodecTagString { get; set; }
        public string? Profile { get; set; }
        public int Width { get; set; }
        public int Height { get; set; }
        public string? PixelFormat { get; set; }
        public string? ColorPrimaries { get; set; }
        public string? ColorTransfer { get; set; }
        public decimal FrameRate { get; set; }
        public List<string> SideDataList { get; set; } = new();
        public long BitRate { get; set; }
        public TimeSpan? Duration { get; set; }
        public Dictionary<string, string> Tags { get; set; } = new();
    }

    private sealed class AudioStream
    {
        public string? CodecName { get; set; }
        public string? CodecTagString { get; set; }
        public string? Profile { get; set; }
        public int Channels { get; set; }
        public string? ChannelLayout { get; set; }
        public string Language { get; set; } = string.Empty;
        public long BitRate { get; set; }
        public TimeSpan? Duration { get; set; }
        public Dictionary<string, string> Tags { get; set; } = new();
    }

    private sealed class SubtitleStream
    {
        public string Language { get; set; } = string.Empty;
    }
}

public static class JsonElementExtensions
{
    public static JsonElement? GetPropertyOrNull(this JsonElement element, string propertyName)
    {
        return element.TryGetProperty(propertyName, out var property) ? property : null;
    }
}
