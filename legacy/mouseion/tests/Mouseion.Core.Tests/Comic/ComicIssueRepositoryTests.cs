// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Comic;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Comic;

public class ComicIssueRepositoryTests : RepositoryTestBase
{
    private readonly ComicSeriesRepository _seriesRepository;
    private readonly ComicIssueRepository _issueRepository;

    public ComicIssueRepositoryTests()
    {
        _seriesRepository = new ComicSeriesRepository(Database);
        _issueRepository = new ComicIssueRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsComicIssue()
    {
        var series = await CreateAndInsertSeriesAsync();
        var issue = CreateComicIssue(series.Id, "The Dark Knight Returns", "1");

        var result = await _issueRepository.InsertAsync(issue);

        Assert.True(result.Id > 0);
        Assert.Equal("The Dark Knight Returns", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsIssueById()
    {
        var series = await CreateAndInsertSeriesAsync();
        var issue = CreateComicIssue(series.Id, "Test Issue");
        var inserted = await _issueRepository.InsertAsync(issue);

        var result = await _issueRepository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
    }

    [Fact]
    public async Task GetBySeriesIdAsync_ReturnsIssuesForSeries()
    {
        var series1 = await CreateAndInsertSeriesAsync("Series 1");
        var series2 = await CreateAndInsertSeriesAsync("Series 2");

        await _issueRepository.InsertAsync(CreateComicIssue(series1.Id, "Issue 1", "1"));
        await _issueRepository.InsertAsync(CreateComicIssue(series1.Id, "Issue 2", "2"));
        await _issueRepository.InsertAsync(CreateComicIssue(series2.Id, "Issue 1", "1"));

        var result = await _issueRepository.GetBySeriesIdAsync(series1.Id);

        Assert.Equal(2, result.Count);
        Assert.All(result, i => Assert.Equal(series1.Id, i.ComicSeriesId));
    }

    [Fact]
    public async Task FindByComicVineIssueIdAsync_ReturnsMatchingIssue()
    {
        var series = await CreateAndInsertSeriesAsync();
        var comicVineIssueId = 123456;
        await _issueRepository.InsertAsync(CreateComicIssue(series.Id, "Test", "1", comicVineIssueId));

        var result = await _issueRepository.FindByComicVineIssueIdAsync(comicVineIssueId);

        Assert.NotNull(result);
        Assert.Equal(comicVineIssueId, result.ComicVineIssueId);
    }

    [Fact]
    public async Task FindByIssueNumberAsync_ReturnsMatchingIssue()
    {
        var series = await CreateAndInsertSeriesAsync();
        await _issueRepository.InsertAsync(CreateComicIssue(series.Id, "Issue 5", "5"));
        await _issueRepository.InsertAsync(CreateComicIssue(series.Id, "Issue 10", "10"));

        var result = await _issueRepository.FindByIssueNumberAsync(series.Id, "5");

        Assert.NotNull(result);
        Assert.Equal("5", result.IssueNumber);
    }

    [Fact]
    public async Task GetUnreadAsync_ReturnsOnlyUnreadIssues()
    {
        var series = await CreateAndInsertSeriesAsync();

        var unread = CreateComicIssue(series.Id, "Unread", "1");
        unread.IsRead = false;
        await _issueRepository.InsertAsync(unread);

        var read = CreateComicIssue(series.Id, "Read", "2");
        read.IsRead = true;
        await _issueRepository.InsertAsync(read);

        var result = await _issueRepository.GetUnreadAsync();

        Assert.Single(result);
        Assert.Equal("Unread", result[0].Title);
    }

    [Fact]
    public async Task MarkReadAsync_SetsIsReadToTrue()
    {
        var series = await CreateAndInsertSeriesAsync();
        var issue = CreateComicIssue(series.Id, "Test");
        issue.IsRead = false;
        var inserted = await _issueRepository.InsertAsync(issue);

        await _issueRepository.MarkReadAsync(inserted.Id);
        var result = await _issueRepository.FindAsync(inserted.Id);

        Assert.True(result!.IsRead);
    }

    [Fact]
    public async Task MarkUnreadAsync_SetsIsReadToFalse()
    {
        var series = await CreateAndInsertSeriesAsync();
        var issue = CreateComicIssue(series.Id, "Test");
        issue.IsRead = true;
        var inserted = await _issueRepository.InsertAsync(issue);

        await _issueRepository.MarkUnreadAsync(inserted.Id);
        var result = await _issueRepository.FindAsync(inserted.Id);

        Assert.False(result!.IsRead);
    }

    [Fact]
    public async Task GetUnreadCountAsync_ReturnsCorrectCount()
    {
        var series = await CreateAndInsertSeriesAsync();

        var unread1 = CreateComicIssue(series.Id, "Unread 1", "1");
        unread1.IsRead = false;
        await _issueRepository.InsertAsync(unread1);

        var unread2 = CreateComicIssue(series.Id, "Unread 2", "2");
        unread2.IsRead = false;
        await _issueRepository.InsertAsync(unread2);

        var read = CreateComicIssue(series.Id, "Read", "3");
        read.IsRead = true;
        await _issueRepository.InsertAsync(read);

        var count = await _issueRepository.GetUnreadCountAsync();

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task GetUnreadCountBySeriesAsync_ReturnsCountForSpecificSeries()
    {
        var series1 = await CreateAndInsertSeriesAsync("Series 1");
        var series2 = await CreateAndInsertSeriesAsync("Series 2");

        var issue1 = CreateComicIssue(series1.Id, "Issue 1", "1");
        issue1.IsRead = false;
        await _issueRepository.InsertAsync(issue1);

        var issue2 = CreateComicIssue(series1.Id, "Issue 2", "2");
        issue2.IsRead = false;
        await _issueRepository.InsertAsync(issue2);

        var issue3 = CreateComicIssue(series2.Id, "Issue 1", "1");
        issue3.IsRead = false;
        await _issueRepository.InsertAsync(issue3);

        var count = await _issueRepository.GetUnreadCountBySeriesAsync(series1.Id);

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task MarkAllReadBySeriesAsync_MarksAllIssuesInSeriesAsRead()
    {
        var series = await CreateAndInsertSeriesAsync();

        var issue1 = CreateComicIssue(series.Id, "Issue 1", "1");
        issue1.IsRead = false;
        await _issueRepository.InsertAsync(issue1);

        var issue2 = CreateComicIssue(series.Id, "Issue 2", "2");
        issue2.IsRead = false;
        await _issueRepository.InsertAsync(issue2);

        await _issueRepository.MarkAllReadBySeriesAsync(series.Id);

        var issues = await _issueRepository.GetBySeriesIdAsync(series.Id);
        Assert.All(issues, i => Assert.True(i.IsRead));
    }

    private async Task<ComicSeries> CreateAndInsertSeriesAsync(string title = "Test Series")
    {
        var series = new ComicSeries
        {
            Title = title,
            ComicVineId = new Random().Next(1, 100000),
            Monitored = true,
            QualityProfileId = 1,
            Added = DateTime.UtcNow
        };
        return await _seriesRepository.InsertAsync(series);
    }

    private static ComicIssue CreateComicIssue(
        int seriesId,
        string? title = null,
        string? issueNumber = null,
        int? comicVineIssueId = null)
    {
        return new ComicIssue
        {
            ComicSeriesId = seriesId,
            Title = title ?? $"Issue #{issueNumber}",
            IssueNumber = issueNumber,
            ComicVineIssueId = comicVineIssueId,
            Added = DateTime.UtcNow
        };
    }
}
