using System.Net;

namespace Mouseion.Api.Tests.TV;

public class EpisodeControllerTests : ControllerTestBase
{
    public EpisodeControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetBySeries_NonExistent_Returns404OrEmpty()
    {
        var response = await Client.GetAsync("/api/v3/episodes/series/99999");
        Assert.True(response.StatusCode is HttpStatusCode.OK or HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task GetBySeriesSeason_NonExistent_Returns404OrEmpty()
    {
        var response = await Client.GetAsync("/api/v3/episodes/series/99999/season/1");
        Assert.True(response.StatusCode is HttpStatusCode.OK or HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task GetById_NonExistent_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/episodes/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
