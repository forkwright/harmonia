// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Albums;
using Mouseion.Api.Artists;
using Mouseion.Api.Books;
using Mouseion.Api.Movies;
using Mouseion.Api.Tracks;

namespace Mouseion.Api.Tests.Common;

public class ControllerParameterValidationTests : ControllerTestBase
{
    public ControllerParameterValidationTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task MovieController_Post_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PostAsJsonAsync("/api/v3/movies", (MovieResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task MovieController_Put_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PutAsJsonAsync("/api/v3/movies/1", (MovieResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BookController_Post_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PostAsJsonAsync("/api/v3/books", (BookResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BookController_Put_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PutAsJsonAsync("/api/v3/books/1", (BookResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task AlbumController_Post_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PostAsJsonAsync("/api/v3/albums", (AlbumResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task AlbumController_Put_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PutAsJsonAsync("/api/v3/albums/1", (AlbumResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }


    [Fact]
    public async Task TrackController_Post_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PostAsJsonAsync("/api/v3/tracks", (TrackResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task TrackController_Put_ReturnsBadRequest_WhenBodyIsNull()
    {
        var response = await Client.PutAsJsonAsync("/api/v3/tracks/1", (TrackResource?)null);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task MovieController_Post_WithEmptyBody_ReturnsBadRequest()
    {
        var content = new StringContent("{}", global::System.Text.Encoding.UTF8, "application/json");
        var response = await Client.PostAsync("/api/v3/movies", content);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BookController_Post_WithEmptyBody_ReturnsBadRequest()
    {
        var content = new StringContent("{}", global::System.Text.Encoding.UTF8, "application/json");
        var response = await Client.PostAsync("/api/v3/books", content);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Theory]
    [InlineData("/api/v3/movies")]
    [InlineData("/api/v3/books")]
    public async Task Controllers_Post_WithNullBody_ReturnsBadRequest(string endpoint)
    {
        var content = new StringContent("null", global::System.Text.Encoding.UTF8, "application/json");
        var response = await Client.PostAsync(endpoint, content);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Theory]
    [InlineData("/api/v3/movies/1")]
    [InlineData("/api/v3/books/1")]
    public async Task Controllers_Put_WithNullBody_ReturnsBadRequest(string endpoint)
    {
        var content = new StringContent("null", global::System.Text.Encoding.UTF8, "application/json");
        var response = await Client.PutAsync(endpoint, content);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

}
