// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications;

public interface INotificationService
{
    Task<IEnumerable<NotificationDefinition>> GetAllAsync(CancellationToken ct = default);
    Task<NotificationDefinition?> GetAsync(int id, CancellationToken ct = default);
    Task<NotificationDefinition> CreateAsync(NotificationDefinition definition, CancellationToken ct = default);
    Task<NotificationDefinition> UpdateAsync(NotificationDefinition definition, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task<bool> TestAsync(int id, CancellationToken ct = default);
}

public partial class NotificationService : INotificationService
{
    private readonly INotificationRepository _repository;
    private readonly INotificationFactory _factory;
    private readonly ILogger<NotificationService> _logger;

    public NotificationService(
        INotificationRepository repository,
        INotificationFactory factory,
        ILogger<NotificationService> logger)
    {
        _repository = repository;
        _factory = factory;
        _logger = logger;
    }

    public async Task<IEnumerable<NotificationDefinition>> GetAllAsync(CancellationToken ct = default)
    {
        return await _repository.AllAsync(ct).ConfigureAwait(false);
    }

    public async Task<NotificationDefinition?> GetAsync(int id, CancellationToken ct = default)
    {
        return await _repository.FindAsync(id, ct).ConfigureAwait(false);
    }

    public async Task<NotificationDefinition> CreateAsync(NotificationDefinition definition, CancellationToken ct = default)
    {
        LogCreatingNotification(definition.Name, definition.Implementation);

        return await _repository.InsertAsync(definition, ct).ConfigureAwait(false);
    }

    public async Task<NotificationDefinition> UpdateAsync(NotificationDefinition definition, CancellationToken ct = default)
    {
        LogUpdatingNotification(definition.Id, definition.Name);

        return await _repository.UpdateAsync(definition, ct).ConfigureAwait(false);
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        LogDeletingNotification(id);

        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
    }

    public async Task<bool> TestAsync(int id, CancellationToken ct = default)
    {
        var definition = await _repository.FindAsync(id, ct).ConfigureAwait(false);

        if (definition == null)
        {
            LogNotificationNotFoundForTest(id);
            return false;
        }

        try
        {
            var notification = _factory.Create(definition);
            var result = await notification.TestAsync().ConfigureAwait(false);

            LogTestNotificationResult(id, definition.Name, result ? "Success" : "Failed");

            return result;
        }
        catch (Exception ex)
        {
            LogTestNotificationFailed(ex, id);
            return false;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Creating notification: {Name} ({Implementation})")]
    private partial void LogCreatingNotification(string name, string implementation);

    [LoggerMessage(Level = LogLevel.Information, Message = "Updating notification {Id}: {Name}")]
    private partial void LogUpdatingNotification(int id, string name);

    [LoggerMessage(Level = LogLevel.Information, Message = "Deleting notification {Id}")]
    private partial void LogDeletingNotification(int id);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Notification {Id} not found for test")]
    private partial void LogNotificationNotFoundForTest(int id);

    [LoggerMessage(Level = LogLevel.Information, Message = "Test notification {Id} ({Name}): {Result}")]
    private partial void LogTestNotificationResult(int id, string name, string result);

    [LoggerMessage(Level = LogLevel.Error, Message = "Test notification {Id} failed with exception")]
    private partial void LogTestNotificationFailed(Exception ex, int id);
}
