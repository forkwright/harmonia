// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Notifications;

public interface INotificationRepository : IBasicRepository<NotificationDefinition>
{
    Task<IEnumerable<NotificationDefinition>> GetEnabledAsync(CancellationToken ct = default);

    Task<IEnumerable<NotificationDefinition>> GetByImplementationAsync(string implementation, CancellationToken ct = default);

    Task<IEnumerable<NotificationDefinition>> GetByTriggerAsync(NotificationTrigger trigger, CancellationToken ct = default);
}

public enum NotificationTrigger
{
    OnGrab,
    OnDownload,
    OnRename,
    OnMediaAdded,
    OnMediaDeleted,
    OnHealthIssue,
    OnHealthRestored,
    OnApplicationUpdate
}
