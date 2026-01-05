// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles.Audio;

public interface IDynamicRangeAnalyzer
{
    Task<int?> AnalyzeAsync(string filePath, CancellationToken ct = default);
    int? Analyze(string filePath);
}

public class DynamicRangeAnalyzer : IDynamicRangeAnalyzer
{
    private readonly ILogger<DynamicRangeAnalyzer> _logger;
    private const int TimeoutMilliseconds = 30000;

    public DynamicRangeAnalyzer(ILogger<DynamicRangeAnalyzer> logger)
    {
        _logger = logger;
    }

    public Task<int?> AnalyzeAsync(string filePath, CancellationToken ct = default)
    {
        return Task.Run(() => Analyze(filePath), ct);
    }

    public int? Analyze(string filePath)
    {
        try
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = "ffmpeg",
                Arguments = $"-i \"{filePath}\" -af astats=measure_overall=Peak_level:measure_perchannel=0 -f null -",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            using var process = new Process { StartInfo = startInfo };
            var output = string.Empty;
            var error = string.Empty;

            process.OutputDataReceived += (sender, e) => { if (e.Data != null) output += e.Data + "\n"; };
            process.ErrorDataReceived += (sender, e) => { if (e.Data != null) error += e.Data + "\n"; };

            process.Start();
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();

            if (!process.WaitForExit(TimeoutMilliseconds))
            {
                process.Kill(entireProcessTree: true);
                _logger.LogWarning("FFmpeg process timed out after {Timeout}ms for file: {Path}", TimeoutMilliseconds, filePath);
                return null;
            }

            var allOutput = output + error;

            var peakMatch = Regex.Match(allOutput, @"Peak level dB:\s*(-?\d+\.?\d*)", RegexOptions.Multiline);
            if (peakMatch.Success && double.TryParse(peakMatch.Groups[1].Value, out var peakLevel))
            {
                var dr = CalculateDR(peakLevel);
                return dr;
            }

            _logger.LogDebug("Could not extract peak level from FFmpeg output for: {Path}", filePath);
            return null;
        }
        catch (System.ComponentModel.Win32Exception ex)
        {
            _logger.LogWarning(ex, "Failed to start FFmpeg process for: {Path}", filePath);
            return null;
        }
        catch (InvalidOperationException ex)
        {
            _logger.LogWarning(ex, "Process error analyzing dynamic range for: {Path}", filePath);
            return null;
        }
        catch (IOException ex)
        {
            _logger.LogWarning(ex, "I/O error analyzing dynamic range for: {Path}", filePath);
            return null;
        }
    }

    private static int CalculateDR(double peakLevelDb)
    {
        var dr = (int)Math.Round(-peakLevelDb);
        return Math.Max(0, Math.Min(dr, 20));
    }
}
