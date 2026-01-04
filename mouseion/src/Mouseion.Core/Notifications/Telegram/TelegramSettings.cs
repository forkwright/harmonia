// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Notifications.Telegram
{
    /// <summary>
    /// Telegram bot notification settings
    /// </summary>
    public class TelegramSettings : NotificationSettings
    {
        /// <summary>
        /// Telegram bot token (from @BotFather)
        /// </summary>
        public string BotToken { get; set; } = string.Empty;

        /// <summary>
        /// Chat ID to send messages to (can be user ID or group/channel ID)
        /// </summary>
        public string ChatId { get; set; } = string.Empty;

        /// <summary>
        /// Send as silent notification (no sound/vibration)
        /// </summary>
        public bool SendSilently { get; set; }
    }
}
