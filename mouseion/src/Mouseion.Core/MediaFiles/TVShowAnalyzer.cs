// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles;

public interface ITVShowAnalyzer
{
    TVEpisodeFileInfo? Analyze(string filePath);
}

public class TVShowAnalyzer : ITVShowAnalyzer
{
    private readonly ILogger<TVShowAnalyzer> _logger;

    // Match S01E01, S01E01E02 (multi-episode), s01e01
    private static readonly Regex SeasonEpisodeRegex = new(
        @"[Ss](\d{1,2})[Ee](\d{1,3})",
        RegexOptions.Compiled);

    // Match quality from filename
    private static readonly Regex QualityRegex = new(
        @"(?:Bluray|BluRay|HDTV|WEB|WEBDL|WEB-DL|Remux|DVDRip|BDRip|BRRip)[-\s]?(\d{3,4}p)?",
        RegexOptions.Compiled | RegexOptions.IgnoreCase);

    // Match "Season ##" folder name
    private static readonly Regex SeasonFolderRegex = new(
        @"[Ss]eason\s*(\d{1,2})",
        RegexOptions.Compiled | RegexOptions.IgnoreCase);

    // Extract episode title: "Show - S01E01 - Episode Title Quality.ext"
    private static readonly Regex EpisodeTitleRegex = new(
        @"[Ss]\d{1,2}[Ee]\d{1,3}(?:[Ee]\d{1,3})?\s*-\s*(.+?)(?:\s+(?:Bluray|BluRay|HDTV|WEB|WEBDL|Remux|DVDRip|BDRip|BRRip)|\.\w{2,4}$)",
        RegexOptions.Compiled | RegexOptions.IgnoreCase);

    public TVShowAnalyzer(ILogger<TVShowAnalyzer> logger)
    {
        _logger = logger;
    }

    public TVEpisodeFileInfo? Analyze(string filePath)
    {
        try
        {
            var fileInfo = new FileInfo(filePath);
            if (!fileInfo.Exists) return null;

            var fileName = Path.GetFileNameWithoutExtension(filePath);

            // Parse S##E##
            var seMatch = SeasonEpisodeRegex.Match(fileName);
            if (!seMatch.Success)
            {
                _logger.LogDebug("Could not parse season/episode from: {FileName}", fileName);
                return null;
            }

            int seasonNumber = int.Parse(seMatch.Groups[1].Value);
            int episodeNumber = int.Parse(seMatch.Groups[2].Value);

            // Get show name from grandparent directory: /Show/Season ##/file.ext
            var seasonDir = Path.GetDirectoryName(filePath) ?? string.Empty;
            var showDir = Path.GetDirectoryName(seasonDir) ?? string.Empty;
            var showName = Path.GetFileName(showDir);

            // Fallback: if the parent isn't a "Season ##" folder, use parent as show name
            if (!SeasonFolderRegex.IsMatch(Path.GetFileName(seasonDir)))
            {
                showName = Path.GetFileName(seasonDir);
            }

            // Extract episode title
            string? episodeTitle = null;
            var titleMatch = EpisodeTitleRegex.Match(fileName);
            if (titleMatch.Success)
            {
                episodeTitle = titleMatch.Groups[1].Value.Trim();
            }

            // Parse quality
            string? quality = null;
            var qualityMatch = QualityRegex.Match(fileName);
            if (qualityMatch.Success)
            {
                quality = qualityMatch.Value;
            }

            return new TVEpisodeFileInfo
            {
                Path = filePath,
                Size = fileInfo.Length,
                ShowName = showName,
                SeasonNumber = seasonNumber,
                EpisodeNumber = episodeNumber,
                EpisodeTitle = episodeTitle,
                Quality = quality,
                Format = fileInfo.Extension.TrimStart('.').ToUpperInvariant()
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to analyze TV file: {Path}", filePath);
            return null;
        }
    }
}

public class TVEpisodeFileInfo
{
    public string Path { get; set; } = string.Empty;
    public long Size { get; set; }
    public string ShowName { get; set; } = string.Empty;
    public int SeasonNumber { get; set; }
    public int EpisodeNumber { get; set; }
    public string? EpisodeTitle { get; set; }
    public string? Quality { get; set; }
    public string? Format { get; set; }
}
