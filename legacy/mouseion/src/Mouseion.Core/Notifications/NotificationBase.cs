// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Threading.Tasks;

namespace Mouseion.Core.Notifications
{
    /// <summary>
    /// Base class for notification implementations with no-op defaults
    /// </summary>
    public abstract class NotificationBase<TSettings> : INotification
        where TSettings : NotificationSettings
    {
        protected TSettings Settings { get; }

        protected NotificationBase(TSettings settings)
        {
            Settings = settings;
        }

        public abstract string Name { get; }
        public abstract string Link { get; }

        public virtual Task<bool> TestAsync()
        {
            return Task.FromResult(true);
        }

        public virtual Task OnGrabAsync(GrabMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnDownloadAsync(DownloadMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnRenameAsync(RenameMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnMediaDeletedAsync(MediaDeletedMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnHealthRestoredAsync(HealthRestoredMessage message)
        {
            return Task.CompletedTask;
        }

        public virtual Task OnApplicationUpdateAsync(ApplicationUpdateMessage message)
        {
            return Task.CompletedTask;
        }

        /// <summary>
        /// Helper to format file sizes
        /// </summary>
        protected static string FormatBytes(long bytes)
        {
            string[] suffixes = { "B", "KB", "MB", "GB", "TB" };
            if (bytes == 0) return "0 B";

            var place = (int)System.Math.Floor(System.Math.Log(bytes, 1024));
            var num = System.Math.Round(bytes / System.Math.Pow(1024, place), 1);
            return $"{num} {suffixes[place]}";
        }
    }
}
