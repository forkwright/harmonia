// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Notifications;
using Xunit;

namespace Mouseion.Core.Tests.Notifications;

public class NotificationRepositoryTests
{
    [Theory]
    [InlineData(NotificationTrigger.OnGrab, "OnGrab")]
    [InlineData(NotificationTrigger.OnDownload, "OnDownload")]
    [InlineData(NotificationTrigger.OnRename, "OnRename")]
    [InlineData(NotificationTrigger.OnMediaAdded, "OnMediaAdded")]
    [InlineData(NotificationTrigger.OnMediaDeleted, "OnMediaDeleted")]
    [InlineData(NotificationTrigger.OnHealthIssue, "OnHealthIssue")]
    [InlineData(NotificationTrigger.OnHealthRestored, "OnHealthRestored")]
    [InlineData(NotificationTrigger.OnApplicationUpdate, "OnApplicationUpdate")]
    public void GetTriggerColumnName_ReturnsCorrectColumn(NotificationTrigger trigger, string expectedColumn)
    {
        var column = NotificationRepository.GetTriggerColumnName(trigger);
        Assert.Equal(expectedColumn, column);
    }

    [Fact]
    public void GetTriggerColumnName_WithInvalidTrigger_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            NotificationRepository.GetTriggerColumnName((NotificationTrigger)999));
    }
}

public class NotificationDefinitionTests
{
    [Fact]
    public void NotificationDefinition_DefaultValues()
    {
        var definition = new NotificationDefinition();

        Assert.Equal(0, definition.Id);
        Assert.Equal(string.Empty, definition.Name);
        Assert.Equal(string.Empty, definition.Implementation);
        Assert.Equal(string.Empty, definition.ConfigContract);
        Assert.Null(definition.Settings);
        Assert.False(definition.Enabled);
        Assert.False(definition.OnGrab);
        Assert.False(definition.OnDownload);
        Assert.False(definition.OnRename);
        Assert.False(definition.OnMediaAdded);
        Assert.False(definition.OnMediaDeleted);
        Assert.False(definition.OnHealthIssue);
        Assert.False(definition.OnHealthRestored);
        Assert.False(definition.OnApplicationUpdate);
        Assert.NotNull(definition.Tags);
        Assert.Empty(definition.Tags);
    }

    [Fact]
    public void NotificationDefinition_CanSetAllProperties()
    {
        var definition = new NotificationDefinition
        {
            Id = 1,
            Name = "Test",
            Implementation = "Discord",
            ConfigContract = "DiscordSettings",
            Settings = "{}",
            Enabled = true,
            OnGrab = true,
            OnDownload = true,
            OnRename = true,
            OnMediaAdded = true,
            OnMediaDeleted = true,
            OnHealthIssue = true,
            OnHealthRestored = true,
            OnApplicationUpdate = true,
            Tags = new HashSet<int> { 1, 2, 3 }
        };

        Assert.Equal(1, definition.Id);
        Assert.Equal("Test", definition.Name);
        Assert.Equal("Discord", definition.Implementation);
        Assert.Equal("DiscordSettings", definition.ConfigContract);
        Assert.Equal("{}", definition.Settings);
        Assert.True(definition.Enabled);
        Assert.True(definition.OnGrab);
        Assert.True(definition.OnDownload);
        Assert.True(definition.OnRename);
        Assert.True(definition.OnMediaAdded);
        Assert.True(definition.OnMediaDeleted);
        Assert.True(definition.OnHealthIssue);
        Assert.True(definition.OnHealthRestored);
        Assert.True(definition.OnApplicationUpdate);
        Assert.Equal(3, definition.Tags.Count);
    }
}

public class NotificationTriggerTests
{
    [Fact]
    public void NotificationTrigger_HasAllExpectedValues()
    {
        var values = Enum.GetValues<NotificationTrigger>();

        Assert.Equal(8, values.Length);
        Assert.Contains(NotificationTrigger.OnGrab, values);
        Assert.Contains(NotificationTrigger.OnDownload, values);
        Assert.Contains(NotificationTrigger.OnRename, values);
        Assert.Contains(NotificationTrigger.OnMediaAdded, values);
        Assert.Contains(NotificationTrigger.OnMediaDeleted, values);
        Assert.Contains(NotificationTrigger.OnHealthIssue, values);
        Assert.Contains(NotificationTrigger.OnHealthRestored, values);
        Assert.Contains(NotificationTrigger.OnApplicationUpdate, values);
    }
}
