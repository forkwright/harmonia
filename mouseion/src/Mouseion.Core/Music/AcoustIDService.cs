// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;

namespace Mouseion.Core.Music;

public class AcoustIDService : IAcoustIDService
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<AcoustIDService> _logger;
    private const string AcoustIDApiUrl = "https://api.acoustid.org/v2/lookup";
    private const string DefaultApiKey = "YOUR_API_KEY"; // User must configure

    public AcoustIDService(IHttpClient httpClient, ILogger<AcoustIDService> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<AcoustIDResult?> LookupAsync(string filePath, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Looking up AcoustID for file: {FilePath}", filePath);

            // Generate fingerprint using chromaprint/fpcalc
            var fingerprint = await GenerateFingerprintAsync(filePath, ct).ConfigureAwait(false);
            if (string.IsNullOrEmpty(fingerprint))
            {
                _logger.LogWarning("Failed to generate fingerprint for {FilePath}", filePath);
                return null;
            }

            // Query AcoustID API
            var requestBuilder = new HttpRequestBuilder(AcoustIDApiUrl);
            requestBuilder.Post();
            requestBuilder.AddFormParameter("client", DefaultApiKey);
            requestBuilder.AddFormParameter("fingerprint", fingerprint);
            requestBuilder.AddFormParameter("duration", "0"); // Duration in seconds (0 = unknown)
            requestBuilder.AddFormParameter("meta", "recordings releasegroups");

            var request = requestBuilder.Build();
            var response = await _httpClient.PostAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("AcoustID API returned {StatusCode}", response.StatusCode);
                return null;
            }

            var result = ParseAcoustIDResponse(response.Content);
            return result;
        }
        catch (System.ComponentModel.Win32Exception ex)
        {
            _logger.LogError(ex, "Failed to start fpcalc process for {FilePath}", filePath);
            return null;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error looking up AcoustID for {FilePath}", filePath);
            return null;
        }
        catch (IOException ex)
        {
            _logger.LogError(ex, "I/O error looking up AcoustID for {FilePath}", filePath);
            return null;
        }
    }

    public async Task<string> GenerateFingerprintAsync(string filePath, CancellationToken ct = default)
    {
        // Placeholder implementation - requires fpcalc binary from chromaprint
        // In production, this would execute: fpcalc -raw -json <filePath>
        // and parse the output to get the fingerprint

        _logger.LogDebug("Generating fingerprint for {FilePath}", filePath);

        // For now, return empty string to indicate fingerprint generation is not yet implemented
        // Full implementation requires:
        // 1. Install chromaprint/fpcalc binary
        // 2. Execute fpcalc process
        // 3. Parse JSON output to extract fingerprint
        await Task.CompletedTask.ConfigureAwait(false);
        return string.Empty;
    }

    private AcoustIDResult? ParseAcoustIDResponse(string jsonContent)
    {
        try
        {
            using var doc = JsonDocument.Parse(jsonContent);
            var root = doc.RootElement;

            if (!root.TryGetProperty("status", out var status) || status.GetString() != "ok")
            {
                _logger.LogWarning("AcoustID response status is not OK");
                return null;
            }

            if (!root.TryGetProperty("results", out var results) || results.GetArrayLength() == 0)
            {
                _logger.LogDebug("No AcoustID results found");
                return null;
            }

            var firstResult = results[0];
            var acoustIDResult = new AcoustIDResult
            {
                Fingerprint = string.Empty,
                Duration = 0,
                Recordings = new List<AcoustIDRecording>()
            };

            if (firstResult.TryGetProperty("recordings", out var recordings))
            {
                foreach (var recording in recordings.EnumerateArray())
                {
                    var acoustIDRecording = new AcoustIDRecording
                    {
                        Id = recording.TryGetProperty("id", out var id) ? id.GetString() ?? string.Empty : string.Empty,
                        Title = recording.TryGetProperty("title", out var title) ? title.GetString() : null,
                        Artists = new List<string>(),
                        Score = firstResult.TryGetProperty("score", out var score) ? score.GetDouble() : 0.0
                    };

                    if (recording.TryGetProperty("artists", out var artists))
                    {
                        foreach (var artist in artists.EnumerateArray())
                        {
                            if (artist.TryGetProperty("name", out var artistName))
                            {
                                var name = artistName.GetString();
                                if (!string.IsNullOrEmpty(name))
                                {
                                    acoustIDRecording.Artists.Add(name);
                                }
                            }
                        }
                    }

                    acoustIDResult.Recordings.Add(acoustIDRecording);
                }
            }

            return acoustIDResult;
        }
        catch (System.Text.Json.JsonException ex)
        {
            _logger.LogError(ex, "JSON parsing error for AcoustID response");
            return null;
        }
        catch (InvalidOperationException ex)
        {
            _logger.LogError(ex, "Invalid AcoustID response format");
            return null;
        }
    }
}
