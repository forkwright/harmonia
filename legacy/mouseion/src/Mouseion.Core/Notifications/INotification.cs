// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Threading.Tasks;

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Interface for notification providers (Discord, Slack, Email, etc.)
    /// </summary>
    public interface INotification
    {
        /// <summary>
        /// Unique identifier for this notification type
        /// </summary>
        string Name { get; }

        /// <summary>
        /// URL to provider's documentation or setup guide
        /// </summary>
        string Link { get; }

        /// <summary>
        /// Send a test notification to verify configuration
        /// </summary>
        Task<bool> TestAsync();

        /// <summary>
        /// Send notification when media is grabbed/queued for download
        /// </summary>
        Task OnGrabAsync(GrabMessage message);

        /// <summary>
        /// Send notification when media download completes and is imported
        /// </summary>
        Task OnDownloadAsync(DownloadMessage message);

        /// <summary>
        /// Send notification when media file is renamed
        /// </summary>
        Task OnRenameAsync(RenameMessage message);

        /// <summary>
        /// Send notification when media is added to library
        /// </summary>
        Task OnMediaAddedAsync(MediaAddedMessage message);

        /// <summary>
        /// Send notification when media is deleted from library
        /// </summary>
        Task OnMediaDeletedAsync(MediaDeletedMessage message);

        /// <summary>
        /// Send notification when health check fails
        /// </summary>
        Task OnHealthIssueAsync(HealthIssueMessage message);

        /// <summary>
        /// Send notification when health check is restored
        /// </summary>
        Task OnHealthRestoredAsync(HealthRestoredMessage message);

        /// <summary>
        /// Send notification when application is updated
        /// </summary>
        Task OnApplicationUpdateAsync(ApplicationUpdateMessage message);
    }
}
