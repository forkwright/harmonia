// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles;

public interface IMovieFileAnalyzer
{
    Task<MovieFileInfo?> AnalyzeAsync(string filePath, CancellationToken ct = default);
    MovieFileInfo? Analyze(string filePath);
}

public class MovieFileAnalyzer : IMovieFileAnalyzer
{
    private readonly ILogger<MovieFileAnalyzer> _logger;

    // Match "Title (Year)" from folder name
    private static readonly Regex TitleYearRegex = new(
        @"^(.+?)\s*\((\d{4})\)",
        RegexOptions.Compiled);

    // Match quality from filename: "Title (Year) - HDTV-1080p.mp4"
    private static readonly Regex QualityRegex = new(
        @"(?:Bluray|BluRay|HDTV|WEB|WEBDL|WEB-DL|Remux|DVDRip|BDRip|BRRip)[-\s]?(\d{3,4}p)?",
        RegexOptions.Compiled | RegexOptions.IgnoreCase);

    private static readonly Regex ResolutionRegex = new(
        @"(\d{3,4})p",
        RegexOptions.Compiled);

    public MovieFileAnalyzer(ILogger<MovieFileAnalyzer> logger)
    {
        _logger = logger;
    }

    public Task<MovieFileInfo?> AnalyzeAsync(string filePath, CancellationToken ct = default)
    {
        return Task.Run(() => Analyze(filePath), ct);
    }

    public MovieFileInfo? Analyze(string filePath)
    {
        try
        {
            var fileInfo = new FileInfo(filePath);
            if (!fileInfo.Exists)
            {
                _logger.LogWarning("Movie file not found: {Path}", filePath);
                return null;
            }

            // Parse folder name for title/year: /media/movies/Title (Year)/filename.ext
            var directory = Path.GetDirectoryName(filePath) ?? string.Empty;
            var folderName = Path.GetFileName(directory);
            var fileName = Path.GetFileNameWithoutExtension(filePath);

            string title = folderName;
            int year = 0;

            var titleMatch = TitleYearRegex.Match(folderName);
            if (titleMatch.Success)
            {
                title = titleMatch.Groups[1].Value.Trim();
                year = int.Parse(titleMatch.Groups[2].Value);
            }

            // Parse quality/resolution from filename
            string? quality = null;
            int? resolution = null;

            var qualityMatch = QualityRegex.Match(fileName);
            if (qualityMatch.Success)
            {
                quality = qualityMatch.Value;
            }

            var resMatch = ResolutionRegex.Match(fileName);
            if (resMatch.Success)
            {
                resolution = int.Parse(resMatch.Groups[1].Value);
            }

            return new MovieFileInfo
            {
                Path = filePath,
                Size = fileInfo.Length,
                Title = title,
                Year = year,
                Quality = quality,
                Resolution = resolution,
                Format = fileInfo.Extension.TrimStart('.').ToUpperInvariant()
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to analyze movie file: {Path}", filePath);
            return null;
        }
    }
}

public class MovieFileInfo
{
    public string Path { get; set; } = string.Empty;
    public long Size { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public string? Quality { get; set; }
    public int? Resolution { get; set; }
    public string? Format { get; set; }
}
