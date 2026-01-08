// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentAssertions;
using FluentValidation.TestHelper;
using Mouseion.Api.Notifications;
using Xunit;

namespace Mouseion.Api.Tests.Notifications;

public class NotificationResourceValidatorTests
{
    private readonly NotificationResourceValidator _validator = new();

    [Fact]
    public void Validate_ValidResource_PassesValidation()
    {
        // Arrange
        var resource = new NotificationResource
        {
            Name = "Discord Notifications",
            Type = "Discord",
            Settings = new Dictionary<string, string> { { "webhookUrl", "https://discord.com/api/webhooks/123" } },
            OnDownload = true
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
        var resource = new NotificationResource
        {
            Name = "",
            Type = "Discord",
            Settings = new Dictionary<string, string>(),
            OnDownload = true
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
        var resource = new NotificationResource
        {
            Name = new string('a', 201),
            Type = "Discord",
            Settings = new Dictionary<string, string>(),
            OnDownload = true
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Name)
            .WithErrorMessage("Name must not exceed 200 characters");
    }

    [Theory]
    [InlineData("Discord")]
    [InlineData("Slack")]
    [InlineData("Telegram")]
    [InlineData("Email")]
    [InlineData("Webhook")]
    [InlineData("Pushover")]
    [InlineData("Gotify")]
    [InlineData("Ntfy")]
    [InlineData("Apprise")]
    [InlineData("discord")] // Case insensitive
    [InlineData("SLACK")] // Case insensitive
    public void Validate_ValidNotificationType_PassesValidation(string type)
    {
        // Arrange
        var resource = new NotificationResource
        {
            Name = "Test",
            Type = type,
            Settings = new Dictionary<string, string>(),
            OnDownload = true
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x.Type);
    }

    [Fact]
    public void Validate_InvalidNotificationType_FailsValidation()
    {
        // Arrange
        var resource = new NotificationResource
        {
            Name = "Test",
            Type = "InvalidType",
            Settings = new Dictionary<string, string>(),
            OnDownload = true
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Type)
            .WithErrorMessage("Type must be one of: Discord, Slack, Telegram, Email, Webhook, Pushover, Gotify, Ntfy, Apprise");
    }

    [Fact]
    public void Validate_NullSettings_FailsValidation()
    {
        // Arrange
        var resource = new NotificationResource
        {
            Name = "Test",
            Type = "Discord",
            Settings = null!,
            OnDownload = true
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x.Settings)
            .WithErrorMessage("Settings are required");
    }

    [Fact]
    public void Validate_NoTriggersEnabled_FailsValidation()
    {
        // Arrange
        var resource = new NotificationResource
        {
            Name = "Test",
            Type = "Discord",
            Settings = new Dictionary<string, string>(),
            OnGrab = false,
            OnDownload = false,
            OnRename = false,
            OnMediaAdded = false,
            OnMediaDeleted = false,
            OnHealthIssue = false,
            OnHealthRestored = false,
            OnApplicationUpdate = false
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldHaveValidationErrorFor(x => x)
            .WithErrorMessage("At least one notification trigger must be enabled");
    }

    [Theory]
    [InlineData(true, false, false, false, false, false, false, false)] // OnGrab
    [InlineData(false, true, false, false, false, false, false, false)] // OnDownload
    [InlineData(false, false, true, false, false, false, false, false)] // OnRename
    [InlineData(false, false, false, true, false, false, false, false)] // OnMediaAdded
    [InlineData(false, false, false, false, true, false, false, false)] // OnMediaDeleted
    [InlineData(false, false, false, false, false, true, false, false)] // OnHealthIssue
    [InlineData(false, false, false, false, false, false, true, false)] // OnHealthRestored
    [InlineData(false, false, false, false, false, false, false, true)] // OnApplicationUpdate
    public void Validate_OneTriggerEnabled_PassesValidation(
        bool onGrab, bool onDownload, bool onRename, bool onMediaAdded,
        bool onMediaDeleted, bool onHealthIssue, bool onHealthRestored, bool onApplicationUpdate)
    {
        // Arrange
        var resource = new NotificationResource
        {
            Name = "Test",
            Type = "Discord",
            Settings = new Dictionary<string, string>(),
            OnGrab = onGrab,
            OnDownload = onDownload,
            OnRename = onRename,
            OnMediaAdded = onMediaAdded,
            OnMediaDeleted = onMediaDeleted,
            OnHealthIssue = onHealthIssue,
            OnHealthRestored = onHealthRestored,
            OnApplicationUpdate = onApplicationUpdate
        };

        // Act
        var result = _validator.TestValidate(resource);

        // Assert
        result.ShouldNotHaveValidationErrorFor(x => x);
    }
}
