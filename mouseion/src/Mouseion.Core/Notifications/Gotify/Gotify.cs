// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications.Gotify
{
    /// <summary>
    /// Gotify notification implementation (self-hosted push notifications)
    /// </summary>
    public partial class Gotify : NotificationBase<GotifySettings>
    {
        private readonly ILogger<Gotify> _logger;
        private readonly HttpClient _httpClient;

        public Gotify(GotifySettings settings, ILogger<Gotify> logger, IHttpClientFactory httpClientFactory)
            : base(settings)
        {
            _logger = logger;
            _httpClient = httpClientFactory.CreateClient();
        }

        public override string Name => "Gotify";
        public override string Link => "https://gotify.net/";

        public override async Task<bool> TestAsync()
        {
            try
            {
                await SendNotificationAsync("Test", $"Test notification from Mouseion at {DateTime.Now:yyyy-MM-dd HH:mm:ss}", Settings.Priority);
                return true;
            }
            catch (HttpRequestException ex)
            {
                LogNetworkError(ex);
                return false;
            }
            catch (TaskCanceledException ex)
            {
                LogRequestTimeout(ex);
                return false;
            }
        }

        public override Task OnGrabAsync(GrabMessage message)
        {
            var text = $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}\n" +
                       $"Indexer: {message.Indexer ?? "Unknown"}";
            return SendNotificationAsync($"Media Grabbed: {message.Title}", text, Settings.Priority);
        }

        public override Task OnDownloadAsync(DownloadMessage message)
        {
            var status = message.IsUpgrade ? "upgraded" : "downloaded";
            var text = $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}";
            return SendNotificationAsync($"Media {status}: {message.Title}", text, Settings.Priority);
        }

        public override Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            var text = $"Type: {message.MediaType}\n" +
                       $"Year: {message.Year}";
            return SendNotificationAsync($"Media Added: {message.Title}", text, Settings.Priority);
        }

        public override Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            var priority = message.Type == "Error" ? 10 : 7;
            return SendNotificationAsync($"Health Check: {message.Source}", message.Message, priority);
        }

        private async Task SendNotificationAsync(string title, string message, int priority)
        {
            var url = $"{Settings.ServerUrl.TrimEnd('/')}/message?token={Settings.AppToken}";
            var payload = new
            {
                title = title,
                message = message,
                priority = priority
            };

            var json = JsonSerializer.Serialize(payload);
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await _httpClient.PostAsync(url, content);
            response.EnsureSuccessStatusCode();

            LogNotificationSent();
        }

        [LoggerMessage(Level = LogLevel.Error, Message = "Network error sending Gotify test notification")]
        private partial void LogNetworkError(Exception ex);

        [LoggerMessage(Level = LogLevel.Warning, Message = "Request timed out or was cancelled sending Gotify test notification")]
        private partial void LogRequestTimeout(Exception ex);

        [LoggerMessage(Level = LogLevel.Debug, Message = "Gotify notification sent successfully")]
        private partial void LogNotificationSent();
    }
}
