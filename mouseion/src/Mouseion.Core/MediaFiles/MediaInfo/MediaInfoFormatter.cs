// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;
using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles.MediaInfo;

public static partial class MediaInfoFormatter
{
    private const string VideoDynamicRangeHdr = "HDR";

    [GeneratedRegex(@"(?<position>^\d\.\d)", RegexOptions.Compiled, 1000)]
    private static partial Regex PositionRegex();

    public static decimal FormatAudioChannels(MediaInfoModel mediaInfo)
    {
        var audioChannels = FormatAudioChannelsFromAudioChannelPositions(mediaInfo);

        if (audioChannels == null || audioChannels == 0.0m)
        {
            audioChannels = mediaInfo.AudioChannels;
        }

        return audioChannels.Value;
    }

    public static string? FormatAudioCodec(MediaInfoModel mediaInfo, string? sceneName, ILogger? logger = null)
    {
        if (mediaInfo.AudioFormat == null)
        {
            return null;
        }

        var audioFormat = mediaInfo.AudioFormat;
        var audioCodecID = mediaInfo.AudioCodecID ?? string.Empty;
        var audioProfile = mediaInfo.AudioProfile ?? string.Empty;

        if (string.IsNullOrEmpty(audioFormat))
        {
            return string.Empty;
        }

        var result = GetAudioCodecName(audioFormat, audioCodecID, audioProfile);

        if (result == null)
        {
            logger?.LogDebug("Unknown audio format: '{AudioFormat}' in '{SceneName}'. Streams: {RawStreamData}",
                audioFormat, sceneName, mediaInfo.RawStreamData);
            return mediaInfo.AudioFormat;
        }

        return result;
    }

    private static string? GetAudioCodecName(string audioFormat, string audioCodecID, string audioProfile)
    {
        if (audioCodecID == "thd+") return "TrueHD Atmos";
        if (audioFormat == "truehd") return "TrueHD";
        if (audioFormat == "flac") return "FLAC";

        if (audioFormat == "dts")
        {
            return GetDtsVariant(audioProfile);
        }

        if (audioCodecID == "ec+3") return "EAC3 Atmos";
        if (audioFormat == "eac3") return "EAC3";
        if (audioFormat == "ac3") return "AC3";

        if (audioFormat == "aac")
        {
            return audioCodecID == "A_AAC/MPEG4/LC/SBR" ? "HE-AAC" : "AAC";
        }

        if (audioFormat == "mp3") return "MP3";
        if (audioFormat == "mp2") return "MP2";
        if (audioFormat == "opus") return "Opus";
        if (audioFormat.StartsWith("pcm_") || audioFormat.StartsWith("adpcm_")) return "PCM";
        if (audioFormat == "vorbis") return "Vorbis";
        if (audioFormat == "wmav1" || audioFormat == "wmav2" || audioFormat == "wmapro") return "WMA";

        return null;
    }

    private static string GetDtsVariant(string audioProfile)
    {
        return audioProfile switch
        {
            "DTS:X" => "DTS-X",
            "DTS-HD MA" => "DTS-HD MA",
            "DTS-ES" => "DTS-ES",
            "DTS-HD HRA" => "DTS-HD HRA",
            "DTS Express" => "DTS Express",
            "DTS 96/24" => "DTS 96/24",
            _ => "DTS"
        };
    }

    public static string? FormatVideoCodec(MediaInfoModel mediaInfo, string? sceneName, ILogger? logger = null)
    {
        if (mediaInfo.VideoFormat == null)
        {
            return null;
        }

        var videoFormat = mediaInfo.VideoFormat;
        var videoCodecID = mediaInfo.VideoCodecID ?? string.Empty;

        if (string.IsNullOrEmpty(videoFormat))
        {
            return videoFormat.Trim();
        }

        var result = GetVideoCodecName(videoFormat, videoCodecID, sceneName);

        if (result == null)
        {
            logger?.LogDebug("Unknown video format: '{VideoFormat}' in '{SceneName}'. Streams: {RawStreamData}",
                videoFormat, sceneName, mediaInfo.RawStreamData);
            return videoFormat.Trim();
        }

        return result;
    }

    private static string? GetVideoCodecName(string videoFormat, string videoCodecID, string? sceneName)
    {
        if (videoCodecID == "x264") return "x264";
        if (videoFormat == "h264") return GetSceneNameMatch(sceneName, "AVC", "x264", "h264");
        if (videoCodecID == "x265") return "x265";
        if (videoFormat == "hevc") return GetSceneNameMatch(sceneName, "HEVC", "x265", "h265");
        if (videoFormat == "mpeg2video") return "MPEG2";
        if (videoFormat == "mpeg1video") return "MPEG";

        if (videoFormat == "mpeg4" || videoFormat.Contains("msmpeg4"))
        {
            return GetMpeg4Variant(videoCodecID);
        }

        if (videoFormat == "vc1") return "VC1";
        if (videoFormat == "av1") return "AV1";
        if (videoFormat.Contains("vp6")) return "VP6";

        if (videoFormat == "vp7" || videoFormat == "vp8" || videoFormat == "vp9")
        {
            return videoFormat.ToUpperInvariant();
        }

        if (videoFormat == "wmv1" || videoFormat == "wmv2" || videoFormat == "wmv3") return "WMV";

        if (IsLegacyCodec(videoFormat)) return "";

        return null;
    }

    private static string GetMpeg4Variant(string videoCodecID)
    {
        if (videoCodecID.ToUpperInvariant() == "XVID") return "XviD";
        if (videoCodecID == "DIV3" || videoCodecID == "DX50" || videoCodecID.ToUpperInvariant() == "DIVX") return "DivX";
        return "";
    }

    private static bool IsLegacyCodec(string videoFormat)
    {
        return videoFormat == "qtrle" ||
               videoFormat == "rpza" ||
               videoFormat == "rv10" ||
               videoFormat == "rv20" ||
               videoFormat == "rv30" ||
               videoFormat == "rv40" ||
               videoFormat == "cinepak" ||
               videoFormat == "rawvideo" ||
               videoFormat == "msvideo1";
    }

    private static decimal? FormatAudioChannelsFromAudioChannelPositions(MediaInfoModel mediaInfo)
    {
        if (mediaInfo.AudioChannelPositions == null)
        {
            return 0;
        }

        var match = PositionRegex().Match(mediaInfo.AudioChannelPositions);
        if (match.Success)
        {
            return decimal.Parse(match.Groups["position"].Value, NumberStyles.Number, CultureInfo.InvariantCulture);
        }

        return 0;
    }

    private static string GetSceneNameMatch(string? sceneName, params string[] tokens)
    {
        sceneName = !string.IsNullOrWhiteSpace(sceneName) ? Path.GetFileNameWithoutExtension(sceneName) : string.Empty;

        var match = tokens.Where(token => sceneName.Contains(token, StringComparison.OrdinalIgnoreCase)).FirstOrDefault();
        return match ?? tokens[^1];
    }

    public static string FormatVideoDynamicRange(MediaInfoModel mediaInfo)
    {
        return mediaInfo.VideoHdrFormat != HdrFormat.None ? VideoDynamicRangeHdr : "";
    }

    public static string FormatVideoDynamicRangeType(MediaInfoModel mediaInfo)
    {
        return mediaInfo.VideoHdrFormat switch
        {
            HdrFormat.DolbyVision => "DV",
            HdrFormat.DolbyVisionHdr10 => "DV HDR10",
            HdrFormat.DolbyVisionHdr10Plus => "DV HDR10Plus",
            HdrFormat.DolbyVisionHlg => "DV HLG",
            HdrFormat.DolbyVisionSdr => "DV SDR",
            HdrFormat.Hdr10 => "HDR10",
            HdrFormat.Hdr10Plus => "HDR10Plus",
            HdrFormat.Hlg10 => "HLG",
            HdrFormat.Pq10 => "PQ",
            _ => ""
        };
    }
}
