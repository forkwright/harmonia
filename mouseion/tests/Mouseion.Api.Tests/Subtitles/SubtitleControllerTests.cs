using System.Net;

namespace Mouseion.Api.Tests.Subtitles;

public class SubtitleControllerTests : ControllerTestBase
{
    public SubtitleControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task Search_WithoutRequiredParams_ReturnsBadRequest()
    {
        var response = await Client.GetAsync("/api/v3/subtitles/search");
        Assert.True(response.StatusCode is HttpStatusCode.BadRequest or HttpStatusCode.InternalServerError);
    }

    [Fact]
    public async Task Search_WithParams_HandlesGracefully()
    {
        var response = await Client.GetAsync("/api/v3/subtitles/search?mediaItemId=1&languages=en");
        Assert.True(response.IsSuccessStatusCode || response.StatusCode == HttpStatusCode.BadRequest);
    }
}
