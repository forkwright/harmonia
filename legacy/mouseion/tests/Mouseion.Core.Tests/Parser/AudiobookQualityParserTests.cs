// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Parser;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.Parser;

public class AudiobookQualityParserTests
{
    [Theory]
    [InlineData("Author - Title (2024) [M4B]", 203)] // M4B
    [InlineData("Author - Title [AAX]", 203)]
    [InlineData("Author - Title (Audible Enhanced)", 203)]
    public void should_parse_m4b_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [MP3 128]", 201)] // MP3_128
    [InlineData("Author - Title [128 kbps]", 201)]
    [InlineData("Author - Title (128k)", 201)]
    [InlineData("Author - Title [AA]", 201)] // Audible AA format
    [InlineData("Author - Title (Audible)", 201)]
    public void should_parse_mp3_128_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [MP3 320]", 202)] // MP3_320
    [InlineData("Author - Title [320 kbps]", 202)]
    [InlineData("Author - Title (V0)", 202)]
    [InlineData("Author - Title [MP3-CBR]", 202)]
    public void should_parse_mp3_320_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [FLAC]", 204)] // AudioFLAC
    [InlineData("Author - Title [FLAC Audiobook]", 204)]
    public void should_parse_flac_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title.m4b", 203)] // M4B
    [InlineData("Author - Title.aax", 203)] // M4B
    [InlineData("Author - Title.aa", 201)] // MP3_128
    public void should_parse_quality_from_extension(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Extension, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title.mp3", 202)] // MP3_320 (default when no quality indicators in name)
    public void should_parse_mp3_files_from_name_when_no_quality_indicators(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        // MP3 extension is ambiguous (could be music), so detection is by Name not Extension
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Fact]
    public void should_return_unknown_for_empty_string()
    {
        var result = AudiobookQualityParser.ParseQuality("");

        Assert.Equal(200, result.Quality.Id); // AudiobookUnknown
    }

    [Fact]
    public void should_return_unknown_for_null_string()
    {
        var result = AudiobookQualityParser.ParseQuality(null);

        Assert.Equal(200, result.Quality.Id); // AudiobookUnknown
    }

    [Theory]
    [InlineData("Author - Title (2024).txt", 200)] // AudiobookUnknown
    [InlineData("Author - Title.jpg", 200)]
    public void should_return_unknown_for_non_audiobook_files(string fileName, int expectedQualityId)
    {
        var result = AudiobookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("/path/to/audiobook/file.m4b", true)]
    [InlineData("/path/to/audiobook/file.aax", true)]
    [InlineData("/path/to/audiobook/file.aa", true)]
    [InlineData("/path/to/audiobook/file.mp3", false)] // MP3 is ambiguous (could be music or audiobook)
    [InlineData("/path/to/audiobook/file.txt", false)]
    [InlineData("/path/to/audiobook/file.jpg", false)]
    public void should_identify_audiobook_files_correctly(string path, bool expectedResult)
    {
        var result = AudiobookQualityParser.IsAudiobookFile(path);

        Assert.Equal(expectedResult, result);
    }

    [Theory]
    [InlineData("Author - Title Audiobook", true)]
    [InlineData("Author - Title Audio Book", true)]
    [InlineData("Author - Title (Unabridged)", true)]
    [InlineData("Author - Title (Abridged)", true)]
    [InlineData("Author - Title Narrated by Jane Doe", true)]
    [InlineData("Author - Title Read by John Smith", true)]
    [InlineData("Author - Title", false)]
    [InlineData("Author - Album", false)]
    public void should_identify_audiobook_indicators(string name, bool expectedResult)
    {
        var result = AudiobookQualityParser.LooksLikeAudiobook(name);

        Assert.Equal(expectedResult, result);
    }
}
