// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Manga;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Manga;

public class MangaChapterRepositoryTests : RepositoryTestBase
{
    private readonly MangaSeriesRepository _seriesRepository;
    private readonly MangaChapterRepository _chapterRepository;

    public MangaChapterRepositoryTests()
    {
        _seriesRepository = new MangaSeriesRepository(Database);
        _chapterRepository = new MangaChapterRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsMangaChapter()
    {
        var series = await CreateAndInsertSeriesAsync();
        var chapter = CreateMangaChapter(series.Id, "Chapter 1", 1);

        var result = await _chapterRepository.InsertAsync(chapter);

        Assert.True(result.Id > 0);
        Assert.Equal("Chapter 1", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsChapterById()
    {
        var series = await CreateAndInsertSeriesAsync();
        var chapter = CreateMangaChapter(series.Id, "Test Chapter");
        var inserted = await _chapterRepository.InsertAsync(chapter);

        var result = await _chapterRepository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
    }

    [Fact]
    public async Task GetBySeriesIdAsync_ReturnsChaptersForSeries()
    {
        var series1 = await CreateAndInsertSeriesAsync("Series 1");
        var series2 = await CreateAndInsertSeriesAsync("Series 2");

        await _chapterRepository.InsertAsync(CreateMangaChapter(series1.Id, "Ch 1", 1));
        await _chapterRepository.InsertAsync(CreateMangaChapter(series1.Id, "Ch 2", 2));
        await _chapterRepository.InsertAsync(CreateMangaChapter(series2.Id, "Ch 1", 1));

        var result = await _chapterRepository.GetBySeriesIdAsync(series1.Id);

        Assert.Equal(2, result.Count);
        Assert.All(result, c => Assert.Equal(series1.Id, c.MangaSeriesId));
    }

    [Fact]
    public async Task FindByMangaDexChapterIdAsync_ReturnsMatchingChapter()
    {
        var series = await CreateAndInsertSeriesAsync();
        var chapterId = "unique-chapter-id-123";
        await _chapterRepository.InsertAsync(CreateMangaChapter(series.Id, "Test", 1, chapterId));

        var result = await _chapterRepository.FindByMangaDexChapterIdAsync(chapterId);

        Assert.NotNull(result);
        Assert.Equal(chapterId, result.MangaDexChapterId);
    }

    [Fact]
    public async Task GetUnreadAsync_ReturnsOnlyUnreadChapters()
    {
        var series = await CreateAndInsertSeriesAsync();

        var unread = CreateMangaChapter(series.Id, "Unread", 1);
        unread.IsRead = false;
        await _chapterRepository.InsertAsync(unread);

        var read = CreateMangaChapter(series.Id, "Read", 2);
        read.IsRead = true;
        await _chapterRepository.InsertAsync(read);

        var result = await _chapterRepository.GetUnreadAsync();

        Assert.Single(result);
        Assert.Equal("Unread", result[0].Title);
    }

    [Fact]
    public async Task MarkReadAsync_SetsIsReadToTrue()
    {
        var series = await CreateAndInsertSeriesAsync();
        var chapter = CreateMangaChapter(series.Id, "Test");
        chapter.IsRead = false;
        var inserted = await _chapterRepository.InsertAsync(chapter);

        await _chapterRepository.MarkReadAsync(inserted.Id);
        var result = await _chapterRepository.FindAsync(inserted.Id);

        Assert.True(result!.IsRead);
    }

    [Fact]
    public async Task GetUnreadCountAsync_ReturnsCorrectCount()
    {
        var series = await CreateAndInsertSeriesAsync();

        var unread1 = CreateMangaChapter(series.Id, "Unread 1", 1);
        unread1.IsRead = false;
        await _chapterRepository.InsertAsync(unread1);

        var unread2 = CreateMangaChapter(series.Id, "Unread 2", 2);
        unread2.IsRead = false;
        await _chapterRepository.InsertAsync(unread2);

        var read = CreateMangaChapter(series.Id, "Read", 3);
        read.IsRead = true;
        await _chapterRepository.InsertAsync(read);

        var count = await _chapterRepository.GetUnreadCountAsync();

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task MarkAllReadBySeriesAsync_MarksAllChaptersInSeriesAsRead()
    {
        var series = await CreateAndInsertSeriesAsync();

        var chapter1 = CreateMangaChapter(series.Id, "Chapter 1", 1);
        chapter1.IsRead = false;
        await _chapterRepository.InsertAsync(chapter1);

        var chapter2 = CreateMangaChapter(series.Id, "Chapter 2", 2);
        chapter2.IsRead = false;
        await _chapterRepository.InsertAsync(chapter2);

        await _chapterRepository.MarkAllReadBySeriesAsync(series.Id);

        var chapters = await _chapterRepository.GetBySeriesIdAsync(series.Id);
        Assert.All(chapters, c => Assert.True(c.IsRead));
    }

    private async Task<MangaSeries> CreateAndInsertSeriesAsync(string title = "Test Series")
    {
        var series = new MangaSeries
        {
            Title = title,
            MangaDexId = Guid.NewGuid().ToString(),
            Monitored = true,
            QualityProfileId = 1,
            Added = DateTime.UtcNow
        };
        return await _seriesRepository.InsertAsync(series);
    }

    private static MangaChapter CreateMangaChapter(
        int seriesId,
        string? title = null,
        decimal? chapterNumber = null,
        string? mangaDexChapterId = null)
    {
        return new MangaChapter
        {
            MangaSeriesId = seriesId,
            Title = title ?? $"Chapter {chapterNumber}",
            ChapterNumber = chapterNumber,
            MangaDexChapterId = mangaDexChapterId ?? Guid.NewGuid().ToString(),
            Added = DateTime.UtcNow
        };
    }
}
