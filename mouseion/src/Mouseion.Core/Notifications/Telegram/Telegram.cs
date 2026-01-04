// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications.Telegram
{
    /// <summary>
    /// Telegram bot notification implementation
    /// </summary>
    public class Telegram : NotificationBase<TelegramSettings>
    {
        private readonly ILogger<Telegram> _logger;
        private readonly HttpClient _httpClient;

        public Telegram(TelegramSettings settings, ILogger<Telegram> logger, IHttpClientFactory httpClientFactory)
            : base(settings)
        {
            _logger = logger;
            _httpClient = httpClientFactory.CreateClient();
        }

        public override string Name => "Telegram";
        public override string Link => "https://core.telegram.org/bots";

        public override async Task<bool> TestAsync()
        {
            try
            {
                await SendMessageAsync($"Test notification from Mouseion at {DateTime.Now:yyyy-MM-dd HH:mm:ss}");
                return true;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Telegram test notification failed");
                return false;
            }
        }

        public override Task OnGrabAsync(GrabMessage message)
        {
            var text = $"<b>{message.Title}</b>\n\n" +
                       $"<b>Media Grabbed</b>\n" +
                       $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}\n" +
                       $"Release Group: {message.ReleaseGroup ?? "Unknown"}\n" +
                       $"Indexer: {message.Indexer ?? "Unknown"}";
            return SendMessageAsync(text);
        }

        public override Task OnDownloadAsync(DownloadMessage message)
        {
            var status = message.IsUpgrade ? "Media Upgraded" : "Media Downloaded";
            var text = $"<b>{message.Title}</b>\n\n" +
                       $"<b>{status}</b>\n" +
                       $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}";
            return SendMessageAsync(text);
        }

        public override Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            var text = $"<b>{message.Title}</b>\n\n" +
                       $"<b>Media Added to Library</b>\n" +
                       $"Type: {message.MediaType}\n" +
                       $"Year: {message.Year}";
            return SendMessageAsync(text);
        }

        public override Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            var text = $"<b>Health Check: {message.Source}</b>\n\n" +
                       $"Type: {message.Type}\n" +
                       $"Message: {message.Message}";
            return SendMessageAsync(text);
        }

        private async Task SendMessageAsync(string text)
        {
            var url = $"https://api.telegram.org/bot{Settings.BotToken}/sendMessage";
            var payload = new
            {
                chat_id = Settings.ChatId,
                text = text,
                parse_mode = "HTML",
                disable_notification = Settings.SendSilently
            };

            var json = JsonSerializer.Serialize(payload);
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await _httpClient.PostAsync(url, content);
            response.EnsureSuccessStatusCode();

            _logger.LogDebug("Telegram notification sent successfully");
        }
    }
}
