// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Tests.Repositories;
using Mouseion.Core.Webcomic;

namespace Mouseion.Core.Tests.Webcomic;

public class WebcomicSeriesRepositoryTests : RepositoryTestBase
{
    private readonly WebcomicSeriesRepository _repository;

    public WebcomicSeriesRepositoryTests()
    {
        _repository = new WebcomicSeriesRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsWebcomicSeries()
    {
        var series = CreateWebcomicSeries("Tower of God", "webtoon-95");

        var result = await _repository.InsertAsync(series);

        Assert.True(result.Id > 0);
        Assert.Equal("Tower of God", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsSeriesById()
    {
        var series = CreateWebcomicSeries("Lore Olympus", "webtoon-1320");
        var inserted = await _repository.InsertAsync(series);

        var result = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
        Assert.Equal("Lore Olympus", result.Title);
    }

    [Fact]
    public async Task FindByWebtoonIdAsync_ReturnsMatchingSeries()
    {
        var webtoonId = "webtoon-unique-123";
        var series = CreateWebcomicSeries("Test Webtoon", webtoonId);
        await _repository.InsertAsync(series);

        var result = await _repository.FindByWebtoonIdAsync(webtoonId);

        Assert.NotNull(result);
        Assert.Equal(webtoonId, result.WebtoonId);
    }

    [Fact]
    public async Task FindByWebtoonIdAsync_ReturnsNullWhenNotFound()
    {
        var result = await _repository.FindByWebtoonIdAsync("nonexistent-id");

        Assert.Null(result);
    }

    [Fact]
    public async Task FindByTapasIdAsync_ReturnsMatchingSeries()
    {
        var tapasId = "tapas-456";
        var series = CreateWebcomicSeries("Tapas Series", tapasId: tapasId);
        await _repository.InsertAsync(series);

        var result = await _repository.FindByTapasIdAsync(tapasId);

        Assert.NotNull(result);
        Assert.Equal(tapasId, result.TapasId);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredSeries()
    {
        var monitoredSeries = CreateWebcomicSeries("Monitored", "w1", monitored: true);
        var unmonitoredSeries = CreateWebcomicSeries("Unmonitored", "w2", monitored: false);
        await _repository.InsertAsync(monitoredSeries);
        await _repository.InsertAsync(unmonitoredSeries);

        var result = await _repository.GetMonitoredAsync();

        Assert.Single(result);
        Assert.Equal("Monitored", result[0].Title);
    }

    [Fact]
    public async Task GetByPlatformAsync_ReturnsSeriesByPlatform()
    {
        var webtoon = CreateWebcomicSeries("Webtoon Series", "w1", platform: "Webtoon");
        var tapas = CreateWebcomicSeries("Tapas Series", tapasId: "t1", platform: "Tapas");
        await _repository.InsertAsync(webtoon);
        await _repository.InsertAsync(tapas);

        var result = await _repository.GetByPlatformAsync("Webtoon");

        Assert.Single(result);
        Assert.Equal("Webtoon Series", result[0].Title);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesSeries()
    {
        var series = CreateWebcomicSeries("Original", "orig-w");
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
        var series = CreateWebcomicSeries("ToDelete", "del-w");
        var inserted = await _repository.InsertAsync(series);

        await _repository.DeleteAsync(inserted.Id);
        var result = await _repository.FindAsync(inserted.Id);

        Assert.Null(result);
    }

    [Fact]
    public void FindByWebtoonId_ReturnsMatchingSeries()
    {
        var webtoonId = "sync-webtoon-id";
        var series = CreateWebcomicSeries("Sync Series", webtoonId);
        _repository.Insert(series);

        var result = _repository.FindByWebtoonId(webtoonId);

        Assert.NotNull(result);
        Assert.Equal(webtoonId, result.WebtoonId);
    }

    private static WebcomicSeries CreateWebcomicSeries(
        string title = "Test Series",
        string? webtoonId = null,
        string? tapasId = null,
        bool monitored = true,
        string? platform = null,
        int qualityProfileId = 1)
    {
        return new WebcomicSeries
        {
            Title = title,
            WebtoonId = webtoonId,
            TapasId = tapasId,
            Monitored = monitored,
            Platform = platform ?? (webtoonId != null ? "Webtoon" : tapasId != null ? "Tapas" : null),
            QualityProfileId = qualityProfileId,
            Added = DateTime.UtcNow
        };
    }
}
