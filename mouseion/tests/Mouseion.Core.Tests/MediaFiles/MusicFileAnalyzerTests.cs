// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.MediaFiles;

public class MusicFileAnalyzerTests
{
    private readonly Mock<ILogger<MusicFileAnalyzer>> _mockLogger;
    private readonly MusicFileAnalyzer _analyzer;

    public MusicFileAnalyzerTests()
    {
        _mockLogger = new Mock<ILogger<MusicFileAnalyzer>>();
        _analyzer = new MusicFileAnalyzer(_mockLogger.Object);
    }

    [Theory]
    [InlineData(22000, 363)] // MusicDSD512
    [InlineData(22579, 363)]
    [InlineData(11289, 362)] // MusicDSD256
    [InlineData(11025, 362)]
    [InlineData(5644, 361)] // MusicDSD128
    [InlineData(5512, 361)]
    [InlineData(2822, 360)] // MusicDSD64
    [InlineData(2000, 360)]
    public void GetDsdQuality_WithSampleRate_ReturnsCorrectQuality(int sampleRateKhz, int expectedQualityId)
    {
        var method = typeof(MusicFileAnalyzer).GetMethod("GetDsdQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (Quality)method!.Invoke(null, new object[] { sampleRateKhz })!;

        Assert.Equal(expectedQualityId, result.Id);
    }

    [Theory]
    [InlineData(24, 192, 327)] // MusicFLAC_24_192
    [InlineData(24, 384, 327)] // Still MusicFLAC_24_192 (highest tier)
    [InlineData(24, 176, 326)] // MusicFLAC_24_176
    [InlineData(24, 96, 325)] // MusicFLAC_24_96
    [InlineData(24, 88, 324)] // MusicFLAC_24_88
    [InlineData(24, 48, 323)] // MusicFLAC_24_48
    [InlineData(24, 44, 322)] // MusicFLAC_24_44
    [InlineData(16, 48, 321)] // MusicFLAC_16_48
    [InlineData(16, 44, 320)] // MusicFLAC_16_44
    [InlineData(16, 32, 320)] // Defaults to MusicFLAC_16_44
    public void GetFlacQuality_WithBitDepthAndSampleRate_ReturnsCorrectQuality(
        int bitDepth, int sampleRateKhz, int expectedQualityId)
    {
        var method = typeof(MusicFileAnalyzer).GetMethod("GetFlacQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (Quality)method!.Invoke(null, new object[] { bitDepth, sampleRateKhz })!;

        Assert.Equal(expectedQualityId, result.Id);
    }

    [Theory]
    [InlineData(24, 192, 347)] // MusicWAV_24_192
    [InlineData(24, 384, 347)] // Still MusicWAV_24_192 (highest tier)
    [InlineData(24, 176, 346)] // MusicWAV_24_176
    [InlineData(24, 96, 345)] // MusicWAV_24_96
    [InlineData(24, 88, 344)] // MusicWAV_24_88
    [InlineData(24, 48, 343)] // MusicWAV_24_48
    [InlineData(24, 44, 342)] // MusicWAV_24_44
    [InlineData(16, 48, 341)] // MusicWAV_16_48
    [InlineData(16, 44, 340)] // MusicWAV_16_44
    [InlineData(16, 32, 340)] // Defaults to MusicWAV_16_44
    public void GetPcmQuality_WithBitDepthAndSampleRate_ReturnsCorrectQuality(
        int bitDepth, int sampleRateKhz, int expectedQualityId)
    {
        var method = typeof(MusicFileAnalyzer).GetMethod("GetPcmQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (Quality)method!.Invoke(null, new object[] { bitDepth, sampleRateKhz })!;

        Assert.Equal(expectedQualityId, result.Id);
    }

    [Theory]
    [InlineData("FLAC", true)]
    [InlineData("PCM", true)]
    [InlineData("APE", true)]
    [InlineData("WavPack", true)]
    [InlineData("DSD", true)]
    [InlineData("MP3", false)]
    [InlineData("AAC", false)]
    [InlineData("Vorbis", false)]
    [InlineData("Opus", false)]
    [InlineData("Unknown", false)]
    public void IsLosslessFormat_WithCodec_ReturnsCorrectResult(string codec, bool expectedResult)
    {
        var method = typeof(MusicFileAnalyzer).GetMethod("IsLosslessFormat",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (bool)method!.Invoke(null, new object[] { codec })!;

        Assert.Equal(expectedResult, result);
    }

    [Fact]
    public void RefineLosslessQuality_WithFlacCodec_RefinesQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "FLAC",
            BitsPerSample = 24,
            SampleRate = 96000
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineLosslessQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicFLAC_16_44, musicInfo })!;

        Assert.Equal(325, result.Id); // MusicFLAC_24_96
    }

    [Fact]
    public void RefineLosslessQuality_WithPcmCodec_RefinesQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "PCM",
            BitsPerSample = 24,
            SampleRate = 192000
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineLosslessQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicWAV_16_44, musicInfo })!;

        Assert.Equal(347, result.Id); // MusicWAV_24_192
    }

    [Fact]
    public void RefineLosslessQuality_WithDsdCodec_RefinesQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "DSD",
            BitsPerSample = 1,
            SampleRate = 11289600 // 11289 kHz
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineLosslessQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicDSD64, musicInfo })!;

        Assert.Equal(362, result.Id); // MusicDSD256
    }

    [Fact]
    public void RefineLosslessQuality_WithApeCodec_ReturnsApeQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "APE",
            BitsPerSample = 16,
            SampleRate = 44100
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineLosslessQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicUnknown, musicInfo })!;

        Assert.Equal(376, result.Id); // MusicAPE
    }

    [Fact]
    public void RefineLosslessQuality_WithWavPackCodec_ReturnsWavPackQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "WavPack",
            BitsPerSample = 16,
            SampleRate = 44100
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineLosslessQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicUnknown, musicInfo })!;

        Assert.Equal(377, result.Id); // MusicWavPack
    }

    [Theory]
    [InlineData("MP3", 320, 304)] // MusicMP3_320
    [InlineData("MP3", 256, 303)] // MusicMP3_256
    [InlineData("MP3", 192, 302)] // MusicMP3_192
    [InlineData("MP3", 128, 301)] // MusicMP3_128
    [InlineData("MP3", 96, 304)] // Defaults to MusicMP3_320
    [InlineData("AAC", 320, 307)] // MusicAAC_320
    [InlineData("AAC", 256, 306)] // MusicAAC_256
    [InlineData("AAC", 128, 305)] // MusicAAC_128
    [InlineData("AAC", 96, 305)] // Defaults to MusicAAC_128
    [InlineData("Vorbis", 320, 311)] // MusicOGG_320
    [InlineData("Vorbis", 256, 310)] // MusicOGG_256
    [InlineData("Vorbis", 192, 309)] // MusicOGG_192
    [InlineData("Vorbis", 128, 308)] // MusicOGG_128
    [InlineData("Opus", 256, 314)] // MusicOpus_256
    [InlineData("Opus", 192, 313)] // MusicOpus_192
    [InlineData("Opus", 128, 312)] // MusicOpus_128
    public void DetermineQualityFromProperties_WithLossyCodec_ReturnsCorrectQuality(
        string codec, int bitrate, int expectedQualityId)
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = codec,
            Bitrate = bitrate
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("DetermineQualityFromProperties",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer, new object[] { musicInfo })!;

        Assert.Equal(expectedQualityId, result.Id);
    }

    [Theory]
    [InlineData("FLAC")]
    [InlineData("PCM")]
    [InlineData("APE")]
    [InlineData("WavPack")]
    [InlineData("DSD")]
    public void DetermineQualityFromProperties_WithLosslessCodec_RefinesQuality(string codec)
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = codec,
            BitsPerSample = 16,
            SampleRate = 44100
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("DetermineQualityFromProperties",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer, new object[] { musicInfo })!;

        Assert.NotEqual(300, result.Id); // Should not be MusicUnknown
    }

    [Fact]
    public void DetermineQualityFromProperties_WithWmaCodec_ReturnsWmaQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "WMA",
            Bitrate = 192
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("DetermineQualityFromProperties",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer, new object[] { musicInfo })!;

        Assert.Equal(315, result.Id); // MusicWMA
    }

    [Fact]
    public void DetermineQualityFromProperties_WithUnknownCodec_ReturnsUnknownQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "Unknown",
            Bitrate = 192
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("DetermineQualityFromProperties",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer, new object[] { musicInfo })!;

        Assert.Equal(300, result.Id); // MusicUnknown
    }

    [Fact]
    public void RefineQuality_WithUnknownFilenameQuality_DeterminesFromProperties()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "MP3",
            Bitrate = 320
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicUnknown, musicInfo })!;

        Assert.Equal(304, result.Id); // MusicMP3_320
    }

    [Fact]
    public void RefineQuality_WithLosslessFormatAndKnownQuality_RefinesQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "FLAC",
            BitsPerSample = 24,
            SampleRate = 192000
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicFLAC_16_44, musicInfo })!;

        Assert.Equal(327, result.Id); // MusicFLAC_24_192 (refined from 16_44)
    }

    [Fact]
    public void RefineQuality_WithLossyFormatAndKnownQuality_ReturnsOriginalQuality()
    {
        var musicInfo = new MusicFileInfo
        {
            Codec = "MP3",
            Bitrate = 320
        };

        var method = typeof(MusicFileAnalyzer).GetMethod("RefineQuality",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (Quality)method!.Invoke(_analyzer,
            new object[] { Quality.MusicMP3_320, musicInfo })!;

        Assert.Equal(304, result.Id); // Returns original MusicMP3_320
    }
}
