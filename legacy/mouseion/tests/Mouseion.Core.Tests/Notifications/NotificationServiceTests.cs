// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.Notifications;
using Xunit;

namespace Mouseion.Core.Tests.Notifications;

public class NotificationServiceTests
{
    private readonly Mock<INotificationRepository> _repositoryMock;
    private readonly Mock<INotificationFactory> _factoryMock;
    private readonly Mock<ILogger<NotificationService>> _loggerMock;
    private readonly NotificationService _service;

    public NotificationServiceTests()
    {
        _repositoryMock = new Mock<INotificationRepository>();
        _factoryMock = new Mock<INotificationFactory>();
        _loggerMock = new Mock<ILogger<NotificationService>>();
        _service = new NotificationService(_repositoryMock.Object, _factoryMock.Object, _loggerMock.Object);
    }

    [Fact]
    public async Task GetAllAsync_ReturnsAllNotifications()
    {
        var notifications = new List<NotificationDefinition>
        {
            new() { Id = 1, Name = "Discord", Implementation = "Discord" },
            new() { Id = 2, Name = "Slack", Implementation = "Slack" }
        };
        _repositoryMock.Setup(r => r.AllAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(notifications);

        var result = await _service.GetAllAsync();

        Assert.Equal(2, result.Count());
        _repositoryMock.Verify(r => r.AllAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task GetAsync_WithValidId_ReturnsNotification()
    {
        var notification = new NotificationDefinition { Id = 1, Name = "Discord" };
        _repositoryMock.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(notification);

        var result = await _service.GetAsync(1);

        Assert.NotNull(result);
        Assert.Equal(1, result.Id);
    }

    [Fact]
    public async Task GetAsync_WithInvalidId_ReturnsNull()
    {
        _repositoryMock.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((NotificationDefinition?)null);

        var result = await _service.GetAsync(999);

        Assert.Null(result);
    }

    [Fact]
    public async Task CreateAsync_InsertsAndReturnsNotification()
    {
        var definition = new NotificationDefinition { Name = "Discord", Implementation = "Discord" };
        var created = new NotificationDefinition { Id = 1, Name = "Discord", Implementation = "Discord" };
        _repositoryMock.Setup(r => r.InsertAsync(definition, It.IsAny<CancellationToken>()))
            .ReturnsAsync(created);

        var result = await _service.CreateAsync(definition);

        Assert.Equal(1, result.Id);
        _repositoryMock.Verify(r => r.InsertAsync(definition, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateAsync_UpdatesAndReturnsNotification()
    {
        var definition = new NotificationDefinition { Id = 1, Name = "Updated Discord", Implementation = "Discord" };
        _repositoryMock.Setup(r => r.UpdateAsync(definition, It.IsAny<CancellationToken>()))
            .ReturnsAsync(definition);

        var result = await _service.UpdateAsync(definition);

        Assert.Equal("Updated Discord", result.Name);
        _repositoryMock.Verify(r => r.UpdateAsync(definition, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteAsync_CallsRepositoryDelete()
    {
        await _service.DeleteAsync(1);

        _repositoryMock.Verify(r => r.DeleteAsync(1, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task TestAsync_WithValidId_ReturnsTrue()
    {
        var definition = new NotificationDefinition { Id = 1, Name = "Discord", Implementation = "Discord" };
        var mockNotification = new Mock<INotification>();
        mockNotification.Setup(n => n.TestAsync()).ReturnsAsync(true);

        _repositoryMock.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(definition);
        _factoryMock.Setup(f => f.Create(definition)).Returns(mockNotification.Object);

        var result = await _service.TestAsync(1);

        Assert.True(result);
    }

    [Fact]
    public async Task TestAsync_WithInvalidId_ReturnsFalse()
    {
        _repositoryMock.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((NotificationDefinition?)null);

        var result = await _service.TestAsync(999);

        Assert.False(result);
    }

    [Fact]
    public async Task TestAsync_WhenExceptionThrown_ReturnsFalse()
    {
        var definition = new NotificationDefinition { Id = 1, Name = "Discord", Implementation = "Discord" };
        _repositoryMock.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(definition);
        _factoryMock.Setup(f => f.Create(definition)).Throws(new InvalidOperationException("Test error"));

        var result = await _service.TestAsync(1);

        Assert.False(result);
    }
}
