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

public class NotificationService : INotificationService
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
        _logger.LogInformation("Creating notification: {Name} ({Implementation})",
            definition.Name, definition.Implementation);

        return await _repository.InsertAsync(definition, ct).ConfigureAwait(false);
    }

    public async Task<NotificationDefinition> UpdateAsync(NotificationDefinition definition, CancellationToken ct = default)
    {
        _logger.LogInformation("Updating notification {Id}: {Name}", definition.Id, definition.Name);

        return await _repository.UpdateAsync(definition, ct).ConfigureAwait(false);
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        _logger.LogInformation("Deleting notification {Id}", id);

        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
    }

    public async Task<bool> TestAsync(int id, CancellationToken ct = default)
    {
        var definition = await _repository.FindAsync(id, ct).ConfigureAwait(false);

        if (definition == null)
        {
            _logger.LogWarning("Notification {Id} not found for test", id);
            return false;
        }

        try
        {
            var notification = _factory.Create(definition);
            var result = await notification.TestAsync().ConfigureAwait(false);

            _logger.LogInformation("Test notification {Id} ({Name}): {Result}",
                id, definition.Name, result ? "Success" : "Failed");

            return result;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Test notification {Id} failed with exception", id);
            return false;
        }
    }
}
