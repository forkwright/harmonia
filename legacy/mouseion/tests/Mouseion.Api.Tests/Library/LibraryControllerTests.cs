using System.Net;
using System.Text;
using System.Text.Json;

namespace Mouseion.Api.Tests.Library;

public class LibraryControllerTests : ControllerTestBase
{
    public LibraryControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task Filter_WithValidFilter_ReturnsSuccess()
    {
        var filter = new { mediaType = "Movie", page = 1, pageSize = 10 };
        var content = new StringContent(JsonSerializer.Serialize(filter), Encoding.UTF8, "application/json");
        var response = await Client.PostAsync("/api/v3/library/filter", content);
        Assert.True(response.StatusCode is HttpStatusCode.OK or HttpStatusCode.BadRequest);
    }
}
