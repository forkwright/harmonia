// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Tests.Repositories;
using Mouseion.Core.Webcomic;

namespace Mouseion.Core.Tests.Webcomic;

public class WebcomicEpisodeRepositoryTests : RepositoryTestBase
{
    private readonly WebcomicSeriesRepository _seriesRepository;
    private readonly WebcomicEpisodeRepository _episodeRepository;

    public WebcomicEpisodeRepositoryTests()
    {
        _seriesRepository = new WebcomicSeriesRepository(Database);
        _episodeRepository = new WebcomicEpisodeRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsWebcomicEpisode()
    {
        var series = await CreateAndInsertSeriesAsync();
        var episode = CreateWebcomicEpisode(series.Id, "Episode 1", 1);

        var result = await _episodeRepository.InsertAsync(episode);

        Assert.True(result.Id > 0);
        Assert.Equal("Episode 1", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsEpisodeById()
    {
        var series = await CreateAndInsertSeriesAsync();
        var episode = CreateWebcomicEpisode(series.Id, "Test Episode");
        var inserted = await _episodeRepository.InsertAsync(episode);

        var result = await _episodeRepository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
    }

    [Fact]
    public async Task GetBySeriesIdAsync_ReturnsEpisodesForSeries()
    {
        var series1 = await CreateAndInsertSeriesAsync("Series 1");
        var series2 = await CreateAndInsertSeriesAsync("Series 2");

        await _episodeRepository.InsertAsync(CreateWebcomicEpisode(series1.Id, "Ep 1", 1));
        await _episodeRepository.InsertAsync(CreateWebcomicEpisode(series1.Id, "Ep 2", 2));
        await _episodeRepository.InsertAsync(CreateWebcomicEpisode(series2.Id, "Ep 1", 1));

        var result = await _episodeRepository.GetBySeriesIdAsync(series1.Id);

        Assert.Equal(2, result.Count);
        Assert.All(result, e => Assert.Equal(series1.Id, e.WebcomicSeriesId));
    }

    [Fact]
    public async Task FindByExternalIdAsync_ReturnsMatchingEpisode()
    {
        var series = await CreateAndInsertSeriesAsync();
        var externalId = "ext-123";
        await _episodeRepository.InsertAsync(CreateWebcomicEpisode(series.Id, "Test", 1, externalId));

        var result = await _episodeRepository.FindByExternalIdAsync(externalId);

        Assert.NotNull(result);
        Assert.Equal(externalId, result.ExternalId);
    }

    [Fact]
    public async Task FindByEpisodeNumberAsync_ReturnsMatchingEpisode()
    {
        var series = await CreateAndInsertSeriesAsync();
        await _episodeRepository.InsertAsync(CreateWebcomicEpisode(series.Id, "Episode 5", 5));
        await _episodeRepository.InsertAsync(CreateWebcomicEpisode(series.Id, "Episode 10", 10));

        var result = await _episodeRepository.FindByEpisodeNumberAsync(series.Id, 5);

        Assert.NotNull(result);
        Assert.Equal(5, result.EpisodeNumber);
    }

    [Fact]
    public async Task GetUnreadAsync_ReturnsOnlyUnreadEpisodes()
    {
        var series = await CreateAndInsertSeriesAsync();

        var unread = CreateWebcomicEpisode(series.Id, "Unread", 1);
        unread.IsRead = false;
        await _episodeRepository.InsertAsync(unread);

        var read = CreateWebcomicEpisode(series.Id, "Read", 2);
        read.IsRead = true;
        await _episodeRepository.InsertAsync(read);

        var result = await _episodeRepository.GetUnreadAsync();

        Assert.Single(result);
        Assert.Equal("Unread", result[0].Title);
    }

    [Fact]
    public async Task MarkReadAsync_SetsIsReadToTrue()
    {
        var series = await CreateAndInsertSeriesAsync();
        var episode = CreateWebcomicEpisode(series.Id, "Test");
        episode.IsRead = false;
        var inserted = await _episodeRepository.InsertAsync(episode);

        await _episodeRepository.MarkReadAsync(inserted.Id);
        var result = await _episodeRepository.FindAsync(inserted.Id);

        Assert.True(result!.IsRead);
    }

    [Fact]
    public async Task MarkUnreadAsync_SetsIsReadToFalse()
    {
        var series = await CreateAndInsertSeriesAsync();
        var episode = CreateWebcomicEpisode(series.Id, "Test");
        episode.IsRead = true;
        var inserted = await _episodeRepository.InsertAsync(episode);

        await _episodeRepository.MarkUnreadAsync(inserted.Id);
        var result = await _episodeRepository.FindAsync(inserted.Id);

        Assert.False(result!.IsRead);
    }

    [Fact]
    public async Task GetUnreadCountAsync_ReturnsCorrectCount()
    {
        var series = await CreateAndInsertSeriesAsync();

        var unread1 = CreateWebcomicEpisode(series.Id, "Unread 1", 1);
        unread1.IsRead = false;
        await _episodeRepository.InsertAsync(unread1);

        var unread2 = CreateWebcomicEpisode(series.Id, "Unread 2", 2);
        unread2.IsRead = false;
        await _episodeRepository.InsertAsync(unread2);

        var read = CreateWebcomicEpisode(series.Id, "Read", 3);
        read.IsRead = true;
        await _episodeRepository.InsertAsync(read);

        var count = await _episodeRepository.GetUnreadCountAsync();

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task GetUnreadCountBySeriesAsync_ReturnsCountForSpecificSeries()
    {
        var series1 = await CreateAndInsertSeriesAsync("Series 1");
        var series2 = await CreateAndInsertSeriesAsync("Series 2");

        var ep1 = CreateWebcomicEpisode(series1.Id, "Ep 1", 1);
        ep1.IsRead = false;
        await _episodeRepository.InsertAsync(ep1);

        var ep2 = CreateWebcomicEpisode(series1.Id, "Ep 2", 2);
        ep2.IsRead = false;
        await _episodeRepository.InsertAsync(ep2);

        var ep3 = CreateWebcomicEpisode(series2.Id, "Ep 1", 1);
        ep3.IsRead = false;
        await _episodeRepository.InsertAsync(ep3);

        var count = await _episodeRepository.GetUnreadCountBySeriesAsync(series1.Id);

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task MarkAllReadBySeriesAsync_MarksAllEpisodesInSeriesAsRead()
    {
        var series = await CreateAndInsertSeriesAsync();

        var episode1 = CreateWebcomicEpisode(series.Id, "Episode 1", 1);
        episode1.IsRead = false;
        await _episodeRepository.InsertAsync(episode1);

        var episode2 = CreateWebcomicEpisode(series.Id, "Episode 2", 2);
        episode2.IsRead = false;
        await _episodeRepository.InsertAsync(episode2);

        await _episodeRepository.MarkAllReadBySeriesAsync(series.Id);

        var episodes = await _episodeRepository.GetBySeriesIdAsync(series.Id);
        Assert.All(episodes, e => Assert.True(e.IsRead));
    }

    private async Task<WebcomicSeries> CreateAndInsertSeriesAsync(string title = "Test Series")
    {
        var series = new WebcomicSeries
        {
            Title = title,
            WebtoonId = Guid.NewGuid().ToString(),
            Monitored = true,
            QualityProfileId = 1,
            Added = DateTime.UtcNow
        };
        return await _seriesRepository.InsertAsync(series);
    }

    private static WebcomicEpisode CreateWebcomicEpisode(
        int seriesId,
        string? title = null,
        int? episodeNumber = null,
        string? externalId = null)
    {
        return new WebcomicEpisode
        {
            WebcomicSeriesId = seriesId,
            Title = title ?? $"Episode {episodeNumber}",
            EpisodeNumber = episodeNumber,
            ExternalId = externalId ?? Guid.NewGuid().ToString(),
            Added = DateTime.UtcNow
        };
    }
}
