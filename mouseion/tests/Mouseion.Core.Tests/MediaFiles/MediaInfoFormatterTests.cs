// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Reflection;
using Mouseion.Core.MediaFiles.MediaInfo;

namespace Mouseion.Core.Tests.MediaFiles;

public class MediaInfoFormatterTests
{
    private static string? InvokeGetAudioCodecName(string audioFormat, string audioCodecID, string audioProfile)
    {
        var method = typeof(MediaInfoFormatter).GetMethod(
            "GetAudioCodecName",
            BindingFlags.NonPublic | BindingFlags.Static);
        return (string?)method?.Invoke(null, new object[] { audioFormat, audioCodecID, audioProfile });
    }

    private static string? InvokeGetVideoCodecName(string videoFormat, string videoCodecID, string? sceneName)
    {
        var method = typeof(MediaInfoFormatter).GetMethod(
            "GetVideoCodecName",
            BindingFlags.NonPublic | BindingFlags.Static);
        return (string?)method?.Invoke(null, new object?[] { videoFormat, videoCodecID, sceneName });
    }

    [Theory]
    [InlineData("", "thd+", "", "TrueHD Atmos")]
    [InlineData("", "ec+3", "", "EAC3 Atmos")]
    public void should_detect_atmos_formats_from_codec_id(string audioFormat, string audioCodecID, string audioProfile, string expected)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("truehd", "", "", "TrueHD")]
    [InlineData("flac", "", "", "FLAC")]
    [InlineData("eac3", "", "", "EAC3")]
    [InlineData("ac3", "", "", "AC3")]
    [InlineData("mp3", "", "", "MP3")]
    [InlineData("mp2", "", "", "MP2")]
    [InlineData("opus", "", "", "Opus")]
    [InlineData("vorbis", "", "", "Vorbis")]
    public void should_detect_audio_codecs_from_format(string audioFormat, string audioCodecID, string audioProfile, string expected)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("aac", "A_AAC/MPEG4/LC/SBR", "", "HE-AAC")]
    [InlineData("aac", "", "", "AAC")]
    [InlineData("aac", "other", "", "AAC")]
    public void should_detect_aac_variants(string audioFormat, string audioCodecID, string audioProfile, string expected)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("dts", "", "DTS:X", "DTS-X")]
    [InlineData("dts", "", "DTS-HD MA", "DTS-HD MA")]
    [InlineData("dts", "", "DTS-ES", "DTS-ES")]
    [InlineData("dts", "", "DTS-HD HRA", "DTS-HD HRA")]
    [InlineData("dts", "", "DTS Express", "DTS Express")]
    [InlineData("dts", "", "DTS 96/24", "DTS 96/24")]
    [InlineData("dts", "", "", "DTS")]
    [InlineData("dts", "", "unknown", "DTS")]
    public void should_detect_dts_variants(string audioFormat, string audioCodecID, string audioProfile, string expected)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("pcm_s16le", "", "", "PCM")]
    [InlineData("pcm_s24le", "", "", "PCM")]
    [InlineData("adpcm_ima_wav", "", "", "PCM")]
    [InlineData("adpcm_ms", "", "", "PCM")]
    public void should_detect_pcm_formats(string audioFormat, string audioCodecID, string audioProfile, string expected)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("wmav1", "", "", "WMA")]
    [InlineData("wmav2", "", "", "WMA")]
    [InlineData("wmapro", "", "", "WMA")]
    public void should_detect_wma_variants(string audioFormat, string audioCodecID, string audioProfile, string expected)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("unknown", "", "")]
    [InlineData("newcodec", "", "")]
    public void should_return_null_for_unknown_audio_format(string audioFormat, string audioCodecID, string audioProfile)
    {
        var result = InvokeGetAudioCodecName(audioFormat, audioCodecID, audioProfile);
        Assert.Null(result);
    }

    [Theory]
    [InlineData("h264", "x264", null, "x264")]
    [InlineData("hevc", "x265", null, "x265")]
    public void should_detect_video_codec_from_codec_id(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("h264", "", "Movie.AVC.1080p", "AVC")]
    [InlineData("h264", "", "Movie.x264.720p", "x264")]
    [InlineData("h264", "", "Movie.1080p", "h264")]
    [InlineData("hevc", "", "Movie.HEVC.2160p", "HEVC")]
    [InlineData("hevc", "", "Movie.x265.1080p", "x265")]
    [InlineData("hevc", "", "Movie.4K", "h265")]
    public void should_detect_video_codec_from_scene_name(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("mpeg2video", "", null, "MPEG2")]
    [InlineData("mpeg1video", "", null, "MPEG")]
    [InlineData("vc1", "", null, "VC1")]
    [InlineData("av1", "", null, "AV1")]
    public void should_detect_video_codecs_from_switch_expression(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("vp6f", "", null, "VP6")]
    [InlineData("vp6a", "", null, "VP6")]
    public void should_detect_vp6_variants(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("vp7", "", null, "VP7")]
    [InlineData("vp8", "", null, "VP8")]
    [InlineData("vp9", "", null, "VP9")]
    public void should_detect_vp_variants_uppercase(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("wmv1", "", null, "WMV")]
    [InlineData("wmv2", "", null, "WMV")]
    [InlineData("wmv3", "", null, "WMV")]
    public void should_detect_wmv_variants(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("mpeg4", "XVID", null, "XviD")]
    [InlineData("mpeg4", "xvid", null, "XviD")]
    [InlineData("mpeg4", "DIV3", null, "DivX")]
    [InlineData("mpeg4", "DX50", null, "DivX")]
    [InlineData("mpeg4", "DIVX", null, "DivX")]
    [InlineData("mpeg4", "divx", null, "DivX")]
    [InlineData("mpeg4", "", null, "")]
    [InlineData("msmpeg4v2", "", null, "")]
    [InlineData("msmpeg4v3", "", null, "")]
    public void should_detect_mpeg4_variants(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("qtrle", "", null, "")]
    [InlineData("rpza", "", null, "")]
    [InlineData("rv10", "", null, "")]
    [InlineData("rv20", "", null, "")]
    [InlineData("rv30", "", null, "")]
    [InlineData("rv40", "", null, "")]
    [InlineData("cinepak", "", null, "")]
    [InlineData("rawvideo", "", null, "")]
    [InlineData("msvideo1", "", null, "")]
    public void should_return_empty_string_for_legacy_codecs(string videoFormat, string videoCodecID, string? sceneName, string expected)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Equal(expected, result);
    }

    [Theory]
    [InlineData("unknown", "", null)]
    [InlineData("newcodec", "", null)]
    public void should_return_null_for_unknown_video_format(string videoFormat, string videoCodecID, string? sceneName)
    {
        var result = InvokeGetVideoCodecName(videoFormat, videoCodecID, sceneName);
        Assert.Null(result);
    }
}
