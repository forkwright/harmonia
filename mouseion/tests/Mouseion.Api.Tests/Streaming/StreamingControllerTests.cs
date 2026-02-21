// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Streaming;
using Mouseion.Core.MediaFiles;

namespace Mouseion.Api.Tests.Streaming;

public class StreamingControllerTests
{
    private readonly Mock<IMediaFileRepository> _mediaFileRepo;
    private readonly StreamingController _controller;

    public StreamingControllerTests()
    {
        _mediaFileRepo = new Mock<IMediaFileRepository>();
        _controller = new StreamingController(_mediaFileRepo.Object);
    }

    [Fact]
    public void StreamMedia_ReturnsNotFound_WhenMediaFileNotInDb()
    {
        _mediaFileRepo.Setup(r => r.Find(42)).Returns((MediaFile?)null);

        var result = _controller.StreamMedia(42);

        Assert.IsType<NotFoundObjectResult>(result);
    }

    [Fact]
    public void StreamMedia_ReturnsNotFound_WhenFileNotOnDisk()
    {
        var mediaFile = new MediaFile { Id = 1, Path = "/nonexistent/path/audio.flac" };
        _mediaFileRepo.Setup(r => r.Find(1)).Returns(mediaFile);

        var result = _controller.StreamMedia(1);

        Assert.IsType<NotFoundObjectResult>(result);
    }

    [Theory]
    [InlineData(".mp3", "audio/mpeg")]
    [InlineData(".flac", "audio/flac")]
    [InlineData(".m4b", "audio/mp4")]
    [InlineData(".m4a", "audio/mp4")]
    [InlineData(".ogg", "audio/ogg")]
    [InlineData(".opus", "audio/opus")]
    [InlineData(".wav", "audio/wav")]
    [InlineData(".aac", "audio/aac")]
    [InlineData(".wma", "audio/x-ms-wma")]
    [InlineData(".mp4", "video/mp4")]
    [InlineData(".mkv", "video/x-matroska")]
    [InlineData(".avi", "video/x-msvideo")]
    [InlineData(".webm", "video/webm")]
    [InlineData(".xyz", "application/octet-stream")]
    public void GetMimeType_ReturnsCorrectType_ForExtension(string extension, string expectedMime)
    {
        var method = typeof(StreamingController).GetMethod(
            "GetMimeType",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);

        Assert.NotNull(method);
        var result = method!.Invoke(null, new object[] { $"test{extension}" });
        Assert.Equal(expectedMime, result);
    }
}
