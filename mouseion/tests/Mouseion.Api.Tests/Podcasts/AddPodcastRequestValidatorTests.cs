// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentAssertions;
using FluentValidation.TestHelper;
using Mouseion.Api.Podcasts;
using Xunit;

namespace Mouseion.Api.Tests.Podcasts;

public class AddPodcastRequestValidatorTests
{
    private readonly AddPodcastRequestValidator _validator = new();

    [Fact]
    public void Validate_ValidRequest_PassesValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            RootFolderPath = "/media/podcasts",
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldNotHaveAnyValidationErrors();
    }

    [Fact]
    public void Validate_EmptyFeedUrl_FailsValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "",
            RootFolderPath = "/media/podcasts",
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.FeedUrl)
            .WithErrorMessage("Feed URL is required");
    }

    [Theory]
    [InlineData("https://example.com/feed.xml")]
    [InlineData("http://example.com/feed.xml")]
    [InlineData("https://subdomain.example.com/path/to/feed.xml")]
    public void Validate_ValidHttpUrl_PassesValidation(string url)
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = url,
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.FeedUrl);
    }

    [Theory]
    [InlineData("ftp://example.com/feed.xml")] // Wrong scheme
    [InlineData("not a url")] // Not a URL
    [InlineData("file:///local/path")] // File scheme
    [InlineData("example.com/feed.xml")] // Missing scheme
    public void Validate_InvalidUrl_FailsValidation(string invalidUrl)
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = invalidUrl,
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.FeedUrl)
            .WithErrorMessage("Feed URL must be a valid URL");
    }

    [Fact]
    public void Validate_QualityProfileIdZero_FailsValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            QualityProfileId = 0
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.QualityProfileId)
            .WithErrorMessage("Quality profile ID must be greater than 0");
    }

    [Fact]
    public void Validate_NegativeQualityProfileId_FailsValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            QualityProfileId = -1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.QualityProfileId)
            .WithErrorMessage("Quality profile ID must be greater than 0");
    }

    [Fact]
    public void Validate_RootFolderPathTooLong_FailsValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            RootFolderPath = new string('a', 1001),
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.RootFolderPath)
            .WithErrorMessage("Root folder path must not exceed 1000 characters");
    }

    [Fact]
    public void Validate_RootFolderPathMaxLength_PassesValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            RootFolderPath = new string('a', 1000),
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.RootFolderPath);
    }

    [Fact]
    public void Validate_NullRootFolderPath_PassesValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            RootFolderPath = null,
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.RootFolderPath);
    }

    [Fact]
    public void Validate_EmptyRootFolderPath_PassesValidation()
    {
        // Arrange
        var request = new AddPodcastRequest
        {
            FeedUrl = "https://example.com/feed.xml",
            RootFolderPath = "",
            QualityProfileId = 1
        };

        // Act
        var result = _validator.TestValidate(request);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.RootFolderPath);
    }
}
