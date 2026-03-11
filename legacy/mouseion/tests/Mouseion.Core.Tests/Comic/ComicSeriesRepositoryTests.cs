// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Comic;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Comic;

public class ComicSeriesRepositoryTests : RepositoryTestBase
{
    private readonly ComicSeriesRepository _repository;

    public ComicSeriesRepositoryTests()
    {
        _repository = new ComicSeriesRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsComicSeries()
    {
        var series = CreateComicSeries("Batman", 18058);

        var result = await _repository.InsertAsync(series);

        Assert.True(result.Id > 0);
        Assert.Equal("Batman", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsSeriesById()
    {
        var series = CreateComicSeries("Spider-Man", 454);
        var inserted = await _repository.InsertAsync(series);

        var result = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
        Assert.Equal("Spider-Man", result.Title);
    }

    [Fact]
    public async Task FindByComicVineIdAsync_ReturnsMatchingSeries()
    {
        var comicVineId = 12345;
        var series = CreateComicSeries("X-Men", comicVineId);
        await _repository.InsertAsync(series);

        var result = await _repository.FindByComicVineIdAsync(comicVineId);

        Assert.NotNull(result);
        Assert.Equal(comicVineId, result.ComicVineId);
    }

    [Fact]
    public async Task FindByComicVineIdAsync_ReturnsNullWhenNotFound()
    {
        var result = await _repository.FindByComicVineIdAsync(99999);

        Assert.Null(result);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredSeries()
    {
        var monitoredSeries = CreateComicSeries("Monitored", 1, monitored: true);
        var unmonitoredSeries = CreateComicSeries("Unmonitored", 2, monitored: false);
        await _repository.InsertAsync(monitoredSeries);
        await _repository.InsertAsync(unmonitoredSeries);

        var result = await _repository.GetMonitoredAsync();

        Assert.Single(result);
        Assert.Equal("Monitored", result[0].Title);
    }

    [Fact]
    public async Task GetByPublisherAsync_ReturnsSeriesByPublisher()
    {
        var dc = CreateComicSeries("Batman", 1, publisher: "DC Comics");
        var marvel = CreateComicSeries("Spider-Man", 2, publisher: "Marvel");
        await _repository.InsertAsync(dc);
        await _repository.InsertAsync(marvel);

        var result = await _repository.GetByPublisherAsync("DC Comics");

        Assert.Single(result);
        Assert.Equal("Batman", result[0].Title);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesSeries()
    {
        var series = CreateComicSeries("Original", 123);
        var inserted = await _repository.InsertAsync(series);

        inserted.Title = "Updated";
        inserted.Monitored = false;
        var result = await _repository.UpdateAsync(inserted);

        Assert.Equal("Updated", result.Title);
        Assert.False(result.Monitored);
    }

    [Fact]
    public async Task DeleteAsync_RemovesSeries()
    {
        var series = CreateComicSeries("ToDelete", 456);
        var inserted = await _repository.InsertAsync(series);

        await _repository.DeleteAsync(inserted.Id);
        var result = await _repository.FindAsync(inserted.Id);

        Assert.Null(result);
    }

    [Fact]
    public void FindByComicVineId_ReturnsMatchingSeries()
    {
        var comicVineId = 78901;
        var series = CreateComicSeries("Sync Series", comicVineId);
        _repository.Insert(series);

        var result = _repository.FindByComicVineId(comicVineId);

        Assert.NotNull(result);
        Assert.Equal(comicVineId, result.ComicVineId);
    }

    private static ComicSeries CreateComicSeries(
        string title = "Test Series",
        int? comicVineId = null,
        string? publisher = null,
        bool monitored = true,
        int qualityProfileId = 1)
    {
        return new ComicSeries
        {
            Title = title,
            ComicVineId = comicVineId,
            Publisher = publisher,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            Added = DateTime.UtcNow
        };
    }
}
