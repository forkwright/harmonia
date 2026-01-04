// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Parser;
using Mouseion.Core.Qualities;
using TagLib;

namespace Mouseion.Core.MediaFiles;

public interface IMusicFileAnalyzer
{
    Task<MusicFileInfo?> AnalyzeAsync(string filePath, CancellationToken ct = default);
    MusicFileInfo? Analyze(string filePath);
}

public class MusicFileAnalyzer : IMusicFileAnalyzer
{
    private readonly ILogger<MusicFileAnalyzer> _logger;

    public MusicFileAnalyzer(ILogger<MusicFileAnalyzer> logger)
    {
        _logger = logger;
    }

    public Task<MusicFileInfo?> AnalyzeAsync(string filePath, CancellationToken ct = default)
    {
        return Task.Run(() => Analyze(filePath), ct);
    }

    public MusicFileInfo? Analyze(string filePath)
    {
        try
        {
            using var file = TagLib.File.Create(filePath);
            var fileInfo = new FileInfo(filePath);

            var musicInfo = new MusicFileInfo
            {
                Path = filePath,
                Size = fileInfo.Length,
                DurationSeconds = (int)file.Properties.Duration.TotalSeconds,
                Bitrate = file.Properties.AudioBitrate,
                SampleRate = file.Properties.AudioSampleRate,
                Channels = file.Properties.AudioChannels,
                BitsPerSample = file.Properties.BitsPerSample,
                Codec = DetermineCodec(file),
                Title = file.Tag.Title,
                Artist = file.Tag.FirstPerformer,
                Album = file.Tag.Album,
                AlbumArtist = file.Tag.FirstAlbumArtist,
                Year = file.Tag.Year,
                TrackNumber = file.Tag.Track,
                DiscNumber = file.Tag.Disc,
                Genre = file.Tag.FirstGenre,
                Comment = file.Tag.Comment
            };

            ExtractMusicBrainzIds(file, musicInfo);
            ExtractReplayGain(file, musicInfo);

            var qualityFromFilename = MusicQualityParser.ParseQuality(filePath, _logger);
            musicInfo.Quality = RefineQuality(qualityFromFilename.Quality, musicInfo);

            return musicInfo;
        }
        catch (TagLib.CorruptFileException ex)
        {
            _logger.LogWarning(ex, "Failed to analyze music file (corrupt): {Path}", filePath);
            return null;
        }
        catch (TagLib.UnsupportedFormatException ex)
        {
            _logger.LogWarning(ex, "Failed to analyze music file (unsupported format): {Path}", filePath);
            return null;
        }
        catch (IOException ex)
        {
            _logger.LogWarning(ex, "Failed to analyze music file (I/O error): {Path}", filePath);
            return null;
        }
    }

    private static string DetermineCodec(TagLib.File file)
    {
        var mimeType = file.MimeType?.ToLowerInvariant() ?? string.Empty;

        return mimeType switch
        {
            "taglib/flac" or "audio/flac" or "audio/x-flac" => "FLAC",
            "taglib/mp3" or "audio/mpeg" or "audio/mp3" => "MP3",
            "taglib/aac" or "audio/aac" or "taglib/m4a" or "audio/mp4" => "AAC",
            "taglib/ogg" or "audio/ogg" or "audio/vorbis" => "Vorbis",
            "taglib/opus" or "audio/opus" => "Opus",
            "taglib/wav" or "audio/wav" or "audio/x-wav" => "PCM",
            "taglib/aiff" or "audio/aiff" or "audio/x-aiff" => "PCM",
            "taglib/ape" or "audio/ape" or "audio/x-ape" => "APE",
            "taglib/wv" or "audio/x-wavpack" => "WavPack",
            "taglib/dsf" or "audio/x-dsf" => "DSD",
            "taglib/dff" or "audio/x-dff" => "DSD",
            "taglib/wma" or "audio/x-ms-wma" => "WMA",
            _ => file.MimeType ?? "Unknown"
        };
    }

    private void ExtractMusicBrainzIds(TagLib.File file, MusicFileInfo musicInfo)
    {
        try
        {
            if (file.Tag is TagLib.Id3v2.Tag id3v2Tag)
            {
                var txxx = TagLib.Id3v2.UserTextInformationFrame.Get(id3v2Tag, "MusicBrainz Album Id", false);
                if (txxx != null && txxx.Text.Length > 0)
                {
                    musicInfo.MusicBrainzReleaseId = txxx.Text[0];
                }

                txxx = TagLib.Id3v2.UserTextInformationFrame.Get(id3v2Tag, "MusicBrainz Artist Id", false);
                if (txxx != null && txxx.Text.Length > 0)
                {
                    musicInfo.MusicBrainzArtistId = txxx.Text[0];
                }

                txxx = TagLib.Id3v2.UserTextInformationFrame.Get(id3v2Tag, "MusicBrainz Release Track Id", false);
                if (txxx != null && txxx.Text.Length > 0)
                {
                    musicInfo.MusicBrainzTrackId = txxx.Text[0];
                }

                txxx = TagLib.Id3v2.UserTextInformationFrame.Get(id3v2Tag, "MusicBrainz Album Artist Id", false);
                if (txxx != null && txxx.Text.Length > 0)
                {
                    musicInfo.MusicBrainzAlbumArtistId = txxx.Text[0];
                }
            }
            else if (file.Tag is TagLib.Ogg.XiphComment vorbisTag)
            {
                musicInfo.MusicBrainzReleaseId = vorbisTag.GetFirstField("MUSICBRAINZ_ALBUMID");
                musicInfo.MusicBrainzArtistId = vorbisTag.GetFirstField("MUSICBRAINZ_ARTISTID");
                musicInfo.MusicBrainzTrackId = vorbisTag.GetFirstField("MUSICBRAINZ_RELEASETRACKID");
                musicInfo.MusicBrainzAlbumArtistId = vorbisTag.GetFirstField("MUSICBRAINZ_ALBUMARTISTID");
            }
            else if (file.Tag is TagLib.Mpeg4.AppleTag mp4Tag)
            {
                musicInfo.MusicBrainzReleaseId = GetAppleTagField(mp4Tag, "----:com.apple.iTunes:MusicBrainz Album Id");
                musicInfo.MusicBrainzArtistId = GetAppleTagField(mp4Tag, "----:com.apple.iTunes:MusicBrainz Artist Id");
                musicInfo.MusicBrainzTrackId = GetAppleTagField(mp4Tag, "----:com.apple.iTunes:MusicBrainz Release Track Id");
                musicInfo.MusicBrainzAlbumArtistId = GetAppleTagField(mp4Tag, "----:com.apple.iTunes:MusicBrainz Album Artist Id");
            }
        }
        catch (InvalidCastException ex)
        {
            _logger.LogDebug(ex, "Failed to extract MusicBrainz IDs from {Path}", musicInfo.Path);
        }
        catch (NullReferenceException ex)
        {
            _logger.LogDebug(ex, "Failed to extract MusicBrainz IDs from {Path}", musicInfo.Path);
        }
    }

    private static string? GetAppleTagField(TagLib.Mpeg4.AppleTag tag, string key)
    {
        return null;
    }

    private void ExtractReplayGain(TagLib.File file, MusicFileInfo musicInfo)
    {
        try
        {
            if (file.Tag is TagLib.Id3v2.Tag id3v2Tag)
            {
                musicInfo.ReplayGainTrackGain = ParseReplayGainValue(GetId3v2TextField(id3v2Tag, "replaygain_track_gain"));
                musicInfo.ReplayGainTrackPeak = ParseReplayGainValue(GetId3v2TextField(id3v2Tag, "replaygain_track_peak"));
                musicInfo.ReplayGainAlbumGain = ParseReplayGainValue(GetId3v2TextField(id3v2Tag, "replaygain_album_gain"));
                musicInfo.ReplayGainAlbumPeak = ParseReplayGainValue(GetId3v2TextField(id3v2Tag, "replaygain_album_peak"));
            }
            else if (file.Tag is TagLib.Ogg.XiphComment vorbisTag)
            {
                musicInfo.ReplayGainTrackGain = ParseReplayGainValue(vorbisTag.GetFirstField("REPLAYGAIN_TRACK_GAIN"));
                musicInfo.ReplayGainTrackPeak = ParseReplayGainValue(vorbisTag.GetFirstField("REPLAYGAIN_TRACK_PEAK"));
                musicInfo.ReplayGainAlbumGain = ParseReplayGainValue(vorbisTag.GetFirstField("REPLAYGAIN_ALBUM_GAIN"));
                musicInfo.ReplayGainAlbumPeak = ParseReplayGainValue(vorbisTag.GetFirstField("REPLAYGAIN_ALBUM_PEAK"));
            }
            else if (file.Tag is TagLib.Ape.Tag apeTag)
            {
                musicInfo.ReplayGainTrackGain = ParseReplayGainValue(GetApeTagField(apeTag, "REPLAYGAIN_TRACK_GAIN"));
                musicInfo.ReplayGainTrackPeak = ParseReplayGainValue(GetApeTagField(apeTag, "REPLAYGAIN_TRACK_PEAK"));
                musicInfo.ReplayGainAlbumGain = ParseReplayGainValue(GetApeTagField(apeTag, "REPLAYGAIN_ALBUM_GAIN"));
                musicInfo.ReplayGainAlbumPeak = ParseReplayGainValue(GetApeTagField(apeTag, "REPLAYGAIN_ALBUM_PEAK"));
            }
        }
        catch (Exception ex)
        {
            _logger.LogDebug(ex, "Failed to extract ReplayGain from {Path}", musicInfo.Path);
        }
    }

    private static string? GetId3v2TextField(TagLib.Id3v2.Tag tag, string description)
    {
        var frame = TagLib.Id3v2.UserTextInformationFrame.Get(tag, description, false);
        return frame != null && frame.Text.Length > 0 ? frame.Text[0] : null;
    }

    private static string? GetApeTagField(TagLib.Ape.Tag tag, string key)
    {
        var item = tag.GetItem(key);
        return item?.ToString();
    }

    private static double? ParseReplayGainValue(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        var cleanValue = value.Trim().Replace(" dB", "").Replace("dB", "");

        if (double.TryParse(cleanValue, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var result))
        {
            return result;
        }

        return null;
    }

    private Quality RefineQuality(Quality filenameQuality, MusicFileInfo musicInfo)
    {
        if (filenameQuality == Quality.MusicUnknown)
        {
            return DetermineQualityFromProperties(musicInfo);
        }

        if (IsLosslessFormat(musicInfo.Codec))
        {
            return RefineLosslessQuality(filenameQuality, musicInfo);
        }

        return filenameQuality;
    }

    private static bool IsLosslessFormat(string codec)
    {
        return codec switch
        {
            "FLAC" or "PCM" or "APE" or "WavPack" or "DSD" => true,
            _ => false
        };
    }

    private Quality RefineLosslessQuality(Quality filenameQuality, MusicFileInfo musicInfo)
    {
        var bitDepth = musicInfo.BitsPerSample;
        var sampleRate = musicInfo.SampleRate / 1000;

        if (musicInfo.Codec == "DSD")
        {
            return sampleRate switch
            {
                >= 22000 => Quality.MusicDSD512,
                >= 11000 => Quality.MusicDSD256,
                >= 5000 => Quality.MusicDSD128,
                _ => Quality.MusicDSD64
            };
        }

        var detectedQuality = (musicInfo.Codec, bitDepth, sampleRate) switch
        {
            ("FLAC", 24, >= 192) => Quality.MusicFLAC_24_192,
            ("FLAC", 24, >= 176) => Quality.MusicFLAC_24_176,
            ("FLAC", 24, >= 96) => Quality.MusicFLAC_24_96,
            ("FLAC", 24, >= 88) => Quality.MusicFLAC_24_88,
            ("FLAC", 24, >= 48) => Quality.MusicFLAC_24_48,
            ("FLAC", 24, >= 44) => Quality.MusicFLAC_24_44,
            ("FLAC", 16, >= 48) => Quality.MusicFLAC_16_48,
            ("FLAC", _, _) => Quality.MusicFLAC_16_44,

            ("PCM", 24, >= 192) => Quality.MusicWAV_24_192,
            ("PCM", 24, >= 176) => Quality.MusicWAV_24_176,
            ("PCM", 24, >= 96) => Quality.MusicWAV_24_96,
            ("PCM", 24, >= 88) => Quality.MusicWAV_24_88,
            ("PCM", 24, >= 48) => Quality.MusicWAV_24_48,
            ("PCM", 24, >= 44) => Quality.MusicWAV_24_44,
            ("PCM", 16, >= 48) => Quality.MusicWAV_16_48,
            ("PCM", _, _) => Quality.MusicWAV_16_44,

            ("APE", _, _) => Quality.MusicAPE,
            ("WavPack", _, _) => Quality.MusicWavPack,

            _ => filenameQuality
        };

        return detectedQuality;
    }

    private Quality DetermineQualityFromProperties(MusicFileInfo musicInfo)
    {
        var codec = musicInfo.Codec;
        var bitrate = musicInfo.Bitrate;

        return codec switch
        {
            "MP3" when bitrate >= 320 => Quality.MusicMP3_320,
            "MP3" when bitrate >= 256 => Quality.MusicMP3_256,
            "MP3" when bitrate >= 192 => Quality.MusicMP3_192,
            "MP3" when bitrate >= 128 => Quality.MusicMP3_128,
            "MP3" => Quality.MusicMP3_320,

            "AAC" when bitrate >= 320 => Quality.MusicAAC_320,
            "AAC" when bitrate >= 256 => Quality.MusicAAC_256,
            "AAC" => Quality.MusicAAC_128,

            "Vorbis" when bitrate >= 320 => Quality.MusicOGG_320,
            "Vorbis" when bitrate >= 256 => Quality.MusicOGG_256,
            "Vorbis" when bitrate >= 192 => Quality.MusicOGG_192,
            "Vorbis" => Quality.MusicOGG_128,

            "Opus" when bitrate >= 256 => Quality.MusicOpus_256,
            "Opus" when bitrate >= 192 => Quality.MusicOpus_192,
            "Opus" => Quality.MusicOpus_128,

            "FLAC" => RefineLosslessQuality(Quality.MusicFLAC_16_44, musicInfo),
            "PCM" => RefineLosslessQuality(Quality.MusicWAV_16_44, musicInfo),
            "APE" => Quality.MusicAPE,
            "WavPack" => Quality.MusicWavPack,
            "DSD" => RefineLosslessQuality(Quality.MusicDSD64, musicInfo),
            "WMA" => Quality.MusicWMA,

            _ => Quality.MusicUnknown
        };
    }
}

public class MusicFileInfo
{
    public string Path { get; set; } = string.Empty;
    public long Size { get; set; }
    public int DurationSeconds { get; set; }
    public int Bitrate { get; set; }
    public int SampleRate { get; set; }
    public int Channels { get; set; }
    public int BitsPerSample { get; set; }
    public string Codec { get; set; } = string.Empty;
    public Quality Quality { get; set; } = Quality.MusicUnknown;

    public string? Title { get; set; }
    public string? Artist { get; set; }
    public string? Album { get; set; }
    public string? AlbumArtist { get; set; }
    public uint Year { get; set; }
    public uint TrackNumber { get; set; }
    public uint DiscNumber { get; set; }
    public string? Genre { get; set; }
    public string? Comment { get; set; }

    public string? MusicBrainzReleaseId { get; set; }
    public string? MusicBrainzArtistId { get; set; }
    public string? MusicBrainzTrackId { get; set; }
    public string? MusicBrainzAlbumArtistId { get; set; }

    public double? ReplayGainTrackGain { get; set; }
    public double? ReplayGainTrackPeak { get; set; }
    public double? ReplayGainAlbumGain { get; set; }
    public double? ReplayGainAlbumPeak { get; set; }
}
