// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Parser;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.Parser;

public class BookQualityParserTests
{
    [Theory]
    [InlineData("Author - Title (2024) [EPUB]", 101)] // EPUB
    [InlineData("Author - Title [ePub]", 101)]
    public void should_parse_epub_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [MOBI]", 102)] // MOBI
    [InlineData("Author - Title [Kindle]", 102)]
    public void should_parse_mobi_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [AZW3]", 103)] // AZW3
    [InlineData("Author - Title [AZW]", 103)]
    [InlineData("Author - Title (Kindle Format 8)", 103)]
    [InlineData("Author - Title [KF8]", 103)]
    public void should_parse_azw3_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [PDF]", 104)] // PDF
    [InlineData("Author - Title [pdf]", 104)]
    public void should_parse_pdf_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title (2024) [TXT]", 105)] // TXT
    [InlineData("Author - Title [Plain Text]", 105)]
    public void should_parse_txt_quality_from_name(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }

    [Theory]
    [InlineData("Author - Title.epub", 101)] // EPUB
    [InlineData("Author - Title.mobi", 102)] // MOBI
    [InlineData("Author - Title.azw", 103)] // AZW3
    [InlineData("Author - Title.azw3", 103)] // AZW3
    [InlineData("Author - Title.pdf", 104)] // PDF
    [InlineData("Author - Title.txt", 105)] // TXT
    public void should_parse_quality_from_extension(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        Assert.Equal(QualityDetectionSource.Extension, result.SourceDetectionSource);
    }

    [Fact]
    public void should_return_unknown_for_empty_string()
    {
        var result = BookQualityParser.ParseQuality("");

        Assert.Equal(100, result.Quality.Id); // EbookUnknown
    }

    [Fact]
    public void should_return_unknown_for_null_string()
    {
        var result = BookQualityParser.ParseQuality(null);

        Assert.Equal(100, result.Quality.Id); // EbookUnknown
    }

    [Theory]
    [InlineData("Author - Title (2024).doc", 100)] // EbookUnknown
    [InlineData("Author - Title.jpg", 100)]
    public void should_return_unknown_for_non_ebook_files(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
    }

    [Theory]
    [InlineData("/path/to/book/file.epub", true)]
    [InlineData("/path/to/book/file.mobi", true)]
    [InlineData("/path/to/book/file.azw", true)]
    [InlineData("/path/to/book/file.azw3", true)]
    [InlineData("/path/to/book/file.pdf", true)]
    [InlineData("/path/to/book/file.txt", true)]
    [InlineData("/path/to/book/file.cbr", true)]
    [InlineData("/path/to/book/file.cbz", true)]
    [InlineData("/path/to/book/file.doc", false)]
    [InlineData("/path/to/book/file.jpg", false)]
    public void should_identify_book_files_correctly(string path, bool expectedResult)
    {
        var result = BookQualityParser.IsBookFile(path);

        Assert.Equal(expectedResult, result);
    }

    [Theory]
    [InlineData("Author - Title [CBR]", 100)] // CBR → EbookUnknown (comic book archive)
    [InlineData("Author - Title [Comic Book RAR]", 100)]
    [InlineData("Author - Title [CBZ]", 100)] // CBZ → EbookUnknown (comic book archive)
    [InlineData("Author - Title [Comic Book ZIP]", 100)]
    public void should_return_unknown_for_comic_book_formats(string fileName, int expectedQualityId)
    {
        var result = BookQualityParser.ParseQuality(fileName);

        Assert.Equal(expectedQualityId, result.Quality.Id);
        // Comic books detected by name but return Unknown (not proper ebook formats)
        Assert.Equal(QualityDetectionSource.Name, result.SourceDetectionSource);
    }
}
