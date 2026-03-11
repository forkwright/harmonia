// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles.Audio;

public class AudioFileAnalyzer : IAudioFileAnalyzer
{
    private readonly ILogger<AudioFileAnalyzer> _logger;

    public AudioFileAnalyzer(ILogger<AudioFileAnalyzer> logger)
    {
        _logger = logger;
    }

    public Task<AudioAnalysisResult> AnalyzeAsync(string filePath, CancellationToken ct = default)
    {
        _logger.LogDebug("Analyzing audio file: {FilePath}", filePath);

        // Placeholder implementation - full spectral analysis requires FFmpeg integration
        // For now, return basic format info from file extension
        var extension = Path.GetExtension(filePath).ToLowerInvariant();
        var result = new AudioAnalysisResult
        {
            Format = extension,
            DeclaredBitDepth = 16,
            DeclaredSampleRate = 44100,
            ActualBitDepth = 16,
            ActualSampleRate = 44100,
            SpectralCeiling = 20000,
            HasUltrasonic = false
        };

        return Task.FromResult(result);
    }

    public bool IsFakeHiRes(AudioAnalysisResult analysis)
    {
        // Fake hi-res detection logic:
        // 1. Declared as 24-bit but actual spectral ceiling suggests 16-bit source
        // 2. Declared as 96kHz+ but no ultrasonic content above 22kHz
        // 3. Hi-res format but spectral ceiling below 20kHz (upsampled)

        return (analysis.DeclaredBitDepth > 16 && analysis.SpectralCeiling < 19000) ||
               (analysis.DeclaredSampleRate >= 88200 && !analysis.HasUltrasonic);
    }
}
