// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Common.Http;
using Mouseion.Core.MetadataSource.TVDB;

namespace Mouseion.Core.Tests.MetadataSource;

public class TVDBClientTests : IDisposable
{
    private readonly Mock<IHttpClient> _httpClientMock;
    private readonly IMemoryCache _cache;
    private readonly TVDBSettings _settings;

    public TVDBClientTests()
    {
        _httpClientMock = new Mock<IHttpClient>();
        _cache = new MemoryCache(new MemoryCacheOptions());
        _settings = new TVDBSettings
        {
            ApiUrl = "https://api4.thetvdb.com/v4",
            ApiKey = "test-api-key",
            TimeoutSeconds = 10,
            MaxRetries = 2
        };
    }

    public void Dispose()
    {
        _cache.Dispose();
    }

    private TVDBClient CreateClient() =>
        new(_settings, _httpClientMock.Object, _cache, NullLogger<TVDBClient>.Instance);

    [Fact]
    public async Task GetAsync_should_authenticate_on_first_call()
    {
        var loginResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": { "token": "test-jwt-token" } }
        """);

        var apiResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": { "id": 81189, "name": "Test" } }
        """);

        _httpClientMock.SetupSequence(c => c.PostAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(loginResponse);

        _httpClientMock.Setup(c => c.GetAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(apiResponse);

        var client = CreateClient();
        var result = await client.GetAsync("series/81189");

        Assert.NotNull(result);
        _httpClientMock.Verify(c => c.PostAsync(It.IsAny<HttpRequest>()), Times.Once);
    }

    [Fact]
    public async Task GetAsync_should_reuse_cached_token()
    {
        var loginResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": { "token": "test-jwt-token" } }
        """);

        var apiResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": {} }
        """);

        _httpClientMock.SetupSequence(c => c.PostAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(loginResponse);

        _httpClientMock.Setup(c => c.GetAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(apiResponse);

        var client = CreateClient();

        // First call authenticates
        await client.GetAsync("series/1");

        // Second call should use cached token
        await client.GetAsync("series/2");

        _httpClientMock.Verify(c => c.PostAsync(It.IsAny<HttpRequest>()), Times.Once);
    }

    [Fact]
    public async Task GetAsync_should_return_null_when_api_key_not_configured()
    {
        _settings.ApiKey = null;
        var client = CreateClient();

        var result = await client.GetAsync("series/81189");

        Assert.Null(result);
        _httpClientMock.Verify(c => c.PostAsync(It.IsAny<HttpRequest>()), Times.Never);
    }

    [Fact]
    public async Task GetAsync_should_return_null_when_authentication_fails()
    {
        var loginResponse = CreateHttpResponse(HttpStatusCode.Unauthorized, "Unauthorized");

        _httpClientMock.Setup(c => c.PostAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(loginResponse);

        var client = CreateClient();
        var result = await client.GetAsync("series/81189");

        Assert.Null(result);
    }

    [Fact]
    public async Task GetAsync_should_return_null_for_non_ok_status()
    {
        var loginResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": { "token": "test-jwt-token" } }
        """);

        var apiResponse = CreateHttpResponse(HttpStatusCode.NotFound, "Not Found");

        _httpClientMock.Setup(c => c.PostAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(loginResponse);

        _httpClientMock.Setup(c => c.GetAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(apiResponse);

        var client = CreateClient();
        var result = await client.GetAsync("series/99999");

        Assert.Null(result);
    }

    [Fact]
    public async Task GetAsync_should_include_bearer_token_in_request()
    {
        var loginResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": { "token": "my-jwt-token" } }
        """);

        var apiResponse = CreateHttpResponse(HttpStatusCode.OK, "{}");

        _httpClientMock.Setup(c => c.PostAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(loginResponse);

        HttpRequest? capturedRequest = null;
        _httpClientMock.Setup(c => c.GetAsync(It.IsAny<HttpRequest>()))
            .Callback<HttpRequest>(r => capturedRequest = r)
            .ReturnsAsync(apiResponse);

        var client = CreateClient();
        await client.GetAsync("series/81189");

        Assert.NotNull(capturedRequest);
        Assert.Contains("Bearer my-jwt-token", capturedRequest.Headers["Authorization"]);
    }

    [Fact]
    public async Task GetAsync_should_construct_correct_url()
    {
        var loginResponse = CreateHttpResponse(HttpStatusCode.OK, """
            { "data": { "token": "token" } }
        """);

        var apiResponse = CreateHttpResponse(HttpStatusCode.OK, "{}");

        _httpClientMock.Setup(c => c.PostAsync(It.IsAny<HttpRequest>()))
            .ReturnsAsync(loginResponse);

        HttpRequest? capturedRequest = null;
        _httpClientMock.Setup(c => c.GetAsync(It.IsAny<HttpRequest>()))
            .Callback<HttpRequest>(r => capturedRequest = r)
            .ReturnsAsync(apiResponse);

        var client = CreateClient();
        await client.GetAsync("series/81189/extended");

        Assert.NotNull(capturedRequest);
        Assert.Equal("https://api4.thetvdb.com/v4/series/81189/extended", capturedRequest.Url.ToString());
    }

    private static HttpResponse CreateHttpResponse(HttpStatusCode statusCode, string content)
    {
        return new HttpResponse(
            new HttpRequest("http://test"),
            new HttpHeader(),
            System.Text.Encoding.UTF8.GetBytes(content),
            statusCode);
    }
}
