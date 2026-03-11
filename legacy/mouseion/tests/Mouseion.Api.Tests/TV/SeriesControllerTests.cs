using System.Net;

namespace Mouseion.Api.Tests.TV;

public class SeriesControllerTests : ControllerTestBase
{
    public SeriesControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetAll_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/series");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetById_NonExistent_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/series/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task GetByTvdbId_NonExistent_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/series/tvdb/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Delete_NonExistent_Returns404()
    {
        var response = await Client.DeleteAsync("/api/v3/series/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
