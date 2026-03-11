// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Parser;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.Parser;

public class MusicQualityParserTests
{
    [Theory]
    [InlineData("Artist - Album (2024) [FLAC 24-192]", 327)] // MusicFLAC_24_192
    [InlineData("Artist - Album [24bit 192khz FLAC]", 327)]
    [InlineData("Artist - Album (24-96)", 325)] // MusicFLAC_24_96
    [InlineData("Artist - Album [FLAC 24-88.2]", 324)] // MusicFLAC_24_88
    [InlineData("Artist - Album (16-44.1 FLAC)", 320)] // MusicFLAC_16_44
    [InlineData("Artist - Album [FLAC]", 320)]
    public void should_parse_flac_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Artist - Album (2024) [DSD512]", 363)] // MusicDSD512
    [InlineData("Artist - Album [DSD256]", 362)] // MusicDSD256
    [InlineData("Artist - Album (DSD128)", 361)] // MusicDSD128
    [InlineData("Artist - Album [DSD64]", 360)] // MusicDSD64
    [InlineData("Artist - Album (DSD)", 360)]
    public void should_parse_dsd_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Artist - Album (2024) [MP3 320]", 304)] // MusicMP3_320
    [InlineData("Artist - Album [320kbps]", 304)]
    [InlineData("Artist - Album (V0)", 304)]
    [InlineData("Artist - Album [MP3-320]", 304)]
    [InlineData("Artist - Album (192)", 302)] // MusicMP3_192
    [InlineData("Artist - Album [MP3-128]", 301)] // MusicMP3_128
    public void should_parse_mp3_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("Artist - Album (2024) [AAC 256]", 306)] // MusicAAC_256
    [InlineData("Artist - Album [AAC-320]", 307)] // MusicAAC_320
    [InlineData("Artist - Album (AAC 128)", 305)] // MusicAAC_128
    public void should_parse_aac_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("Artist - Album (2024) [Opus 192]", 313)] // MusicOpus_192
    [InlineData("Artist - Album [Opus-256]", 314)] // MusicOpus_256
    [InlineData("Artist - Album (Opus 128)", 312)] // MusicOpus_128
    public void should_parse_opus_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("Artist - Album (2024) [MQA Studio]", 381)] // MusicMQA_Studio
    [InlineData("Artist - Album [MQA]", 380)] // MusicMQA
    public void should_parse_mqa_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("Artist - Album.flac", 320)] // MusicFLAC_16_44
    [InlineData("Artist - Album.mp3", 304)] // MusicMP3_320
    [InlineData("Artist - Album.aac", 306)] // MusicAAC_256
    [InlineData("Artist - Album.opus", 313)] // MusicOpus_192
    [InlineData("Artist - Album.wav", 340)] // MusicWAV_16_44
    [InlineData("Artist - Album.aiff", 350)] // MusicAIFF_16_44
    [InlineData("Artist - Album.ape", 376)] // MusicAPE
    [InlineData("Artist - Album.wv", 377)] // MusicWavPack
    [InlineData("Artist - Album.dsf", 360)] // MusicDSD64
    public void should_parse_quality_from_extension(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Extension, result.SourceDetectionSource);
    }

    [Fact]
    public void should_return_unknown_for_empty_string()
    {
        var result = MusicQualityParser.ParseQuality("");

        Assert.Equal(300, result.Quality.Id); // MusicUnknown
    }

    [Fact]
    public void should_return_unknown_for_null_string()
    {
        var result = MusicQualityParser.ParseQuality(null);

        Assert.Equal(300, result.Quality.Id); // MusicUnknown
    }

    [Theory]
    [InlineData("Artist - Album (2024).txt", 300)] // MusicUnknown
    [InlineData("Artist - Album.jpg", 300)]
    public void should_return_unknown_for_non_music_files(string fileName, int expectedQualityId)
    {
        var result = MusicQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("/path/to/music/file.flac", true)]
    [InlineData("/path/to/music/file.mp3", true)]
    [InlineData("/path/to/music/file.wav", true)]
    [InlineData("/path/to/music/file.txt", false)]
    [InlineData("/path/to/music/file.jpg", false)]
    public void should_identify_music_files_correctly(string path, bool expectedResult)
    {
        var result = MusicQualityParser.IsMusicFile(path);

        Assert.Equal(expectedResult, result);
    }
}
