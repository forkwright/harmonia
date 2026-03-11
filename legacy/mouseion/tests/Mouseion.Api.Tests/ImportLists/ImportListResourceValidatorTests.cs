// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentAssertions;
using FluentValidation.TestHelper;
using Mouseion.Api.ImportLists;
using Xunit;

namespace Mouseion.Api.Tests.ImportLists;

public class ImportListResourceValidatorTests
{
    private readonly ImportListResourceValidator _validator = new();

    [Fact]
    public void Validate_ValidResource_PassesValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "My Import List",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = "{\"apiKey\": \"test123\"}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldNotHaveAnyValidationErrors();
    }

    [Fact]
    public void Validate_EmptyName_FailsValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Name)
            .WithErrorMessage("Name is required");
    }

    [Fact]
    public void Validate_NameTooLong_FailsValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = new string('a', 201),
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Name)
            .WithErrorMessage("Name must not exceed 200 characters");
    }

    [Fact]
    public void Validate_EmptyImplementation_FailsValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Implementation)
            .WithErrorMessage("Implementation is required");
    }

    [Fact]
    public void Validate_QualityProfileIdZero_FailsValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "Goodreads",
            QualityProfileId = 0,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.QualityProfileId)
            .WithErrorMessage("Quality profile ID must be greater than 0");
    }

    [Fact]
    public void Validate_EmptyRootFolderPath_FailsValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.RootFolderPath)
            .WithErrorMessage("Root folder path is required");
    }

    [Fact]
    public void Validate_RefreshIntervalTooShort_FailsValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromMinutes(4),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.MinRefreshInterval)
            .WithErrorMessage("Minimum refresh interval must be at least 5 minutes");
    }

    [Fact]
    public void Validate_MinimumRefreshInterval_PassesValidation()
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromMinutes(5),
            Settings = "{}"
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.MinRefreshInterval);
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData("not json")]
    [InlineData("{invalid}")]
    public void Validate_InvalidJsonSettings_FailsValidation(string invalidSettings)
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = invalidSettings
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Settings)
            .WithErrorMessage("Settings must be valid JSON");
    }

    [Theory]
    [InlineData("{}")]
    [InlineData("{\"key\": \"value\"}")]
    [InlineData("{\"nested\": {\"key\": \"value\"}}")]
    public void Validate_ValidJsonSettings_PassesValidation(string validSettings)
    {
        // Arrange
        var resource = new ImportListResource
        {
            Name = "Test",
            Implementation = "Goodreads",
            QualityProfileId = 1,
            RootFolderPath = "/media/books",
            MinRefreshInterval = TimeSpan.FromHours(1),
            Settings = validSettings
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.Settings);
    }
}
