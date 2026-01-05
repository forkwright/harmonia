// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications.Apprise
{
    /// <summary>
    /// Apprise notification implementation (universal notification gateway)
    /// </summary>
    public class Apprise : NotificationBase<AppriseSettings>
    {
        private readonly ILogger<Apprise> _logger;
        private readonly HttpClient _httpClient;

        public Apprise(AppriseSettings settings, ILogger<Apprise> logger, IHttpClientFactory httpClientFactory)
            : base(settings)
        {
            _logger = logger;
            _httpClient = httpClientFactory.CreateClient();
        }

        public override string Name => "Apprise";
        public override string Link => "https://github.com/caronc/apprise";

        public override async Task<bool> TestAsync()
        {
            try
            {
                await SendNotificationAsync("Test", $"Test notification from Mouseion at {DateTime.Now:yyyy-MM-dd HH:mm:ss}");
                return true;
            }
            catch (HttpRequestException ex)
            {
                _logger.LogError(ex, "Network error sending Apprise test notification");
                return false;
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Invalid Apprise configuration");
                return false;
            }
            catch (TaskCanceledException ex)
            {
                _logger.LogWarning(ex, "Request timed out or was cancelled sending Apprise test notification");
                return false;
            }
        }

        public override Task OnGrabAsync(GrabMessage message)
        {
            var body = $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}\n" +
                       $"Indexer: {message.Indexer ?? "Unknown"}";
            return SendNotificationAsync($"Media Grabbed: {message.Title}", body);
        }

        public override Task OnDownloadAsync(DownloadMessage message)
        {
            var status = message.IsUpgrade ? "upgraded" : "downloaded";
            var body = $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}";
            return SendNotificationAsync($"Media {status}: {message.Title}", body);
        }

        public override Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            var body = $"Type: {message.MediaType}\n" +
                       $"Year: {message.Year}";
            return SendNotificationAsync($"Media Added: {message.Title}", body);
        }

        public override Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            return SendNotificationAsync($"Health Check: {message.Source}", message.Message);
        }

        private async Task SendNotificationAsync(string title, string body)
        {
            var url = $"{Settings.ServerUrl.TrimEnd('/')}/notify";

            object payload;
            if (!string.IsNullOrWhiteSpace(Settings.ConfigurationKey))
            {
                // Stateful mode with configuration key
                payload = new
                {
                    urls = Settings.ConfigurationKey,
                    title = title,
                    body = body,
                    tag = Settings.Tags
                };
            }
            else if (!string.IsNullOrWhiteSpace(Settings.NotificationUrls))
            {
                // Stateless mode with notification URLs
                var urls = Settings.NotificationUrls.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
                payload = new
                {
                    urls = urls,
                    title = title,
                    body = body,
                    tag = Settings.Tags
                };
            }
            else
            {
                throw new InvalidOperationException("Either ConfigurationKey or NotificationUrls must be set");
            }

            var json = JsonSerializer.Serialize(payload, new JsonSerializerOptions
            {
                DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull
            });
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await _httpClient.PostAsync(url, content);
            response.EnsureSuccessStatusCode();

            _logger.LogDebug("Apprise notification sent successfully");
        }
    }
}
