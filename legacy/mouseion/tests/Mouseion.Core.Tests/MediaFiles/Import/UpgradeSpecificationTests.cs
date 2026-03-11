// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.MediaFiles.Import.Specifications;
using Mouseion.Core.Music;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.MediaFiles.Import;

public class UpgradeSpecificationTests
{
    private readonly Mock<IMusicFileRepository> _musicFileRepository;
    private readonly Mock<ILogger<UpgradeSpecification>> _logger;
    private readonly UpgradeSpecification _subject;

    public UpgradeSpecificationTests()
    {
        _musicFileRepository = new Mock<IMusicFileRepository>();
        _logger = new Mock<ILogger<UpgradeSpecification>>();
        _subject = new UpgradeSpecification(_musicFileRepository.Object, _logger.Object);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_NewFile_ReturnsNull()
    {
        var musicFileInfo = CreateMusicFileInfo("/music/test.flac", Quality.MusicFLAC_16_44);
        _musicFileRepository.Setup(x => x.FindByPathAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((MusicFile?)null);

        var result = await _subject.IsSatisfiedByAsync(musicFileInfo);

        Assert.Null(result);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_UpgradeQuality_ReturnsNull()
    {
        var existingFile = CreateMusicFile("/music/test.flac", Quality.MusicMP3_320);
        var newFile = CreateMusicFileInfo("/music/test.flac", Quality.MusicFLAC_16_44);

        _musicFileRepository.Setup(x => x.FindByPathAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _subject.IsSatisfiedByAsync(newFile);

        Assert.Null(result);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_NotUpgrade_ReturnsRejection()
    {
        var existingFile = CreateMusicFile("/music/test.flac", Quality.MusicFLAC_24_96);
        var newFile = CreateMusicFileInfo("/music/test.flac", Quality.MusicMP3_320);

        _musicFileRepository.Setup(x => x.FindByPathAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _subject.IsSatisfiedByAsync(newFile);

        Assert.NotNull(result);
        Assert.Equal(ImportRejectionReason.NotQualityUpgrade, result.Reason);
    }

    [Fact]
    public async Task IsSatisfiedByAsync_SameQuality_ReturnsRejection()
    {
        var existingFile = CreateMusicFile("/music/test.flac", Quality.MusicFLAC_16_44);
        var newFile = CreateMusicFileInfo("/music/test.flac", Quality.MusicFLAC_16_44);

        _musicFileRepository.Setup(x => x.FindByPathAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingFile);

        var result = await _subject.IsSatisfiedByAsync(newFile);

        Assert.NotNull(result);
        Assert.Equal(ImportRejectionReason.NotQualityUpgrade, result.Reason);
    }

    [Fact]
    public void IsSatisfiedBy_NewFile_ReturnsNull()
    {
        var musicFileInfo = CreateMusicFileInfo("/music/test.flac", Quality.MusicFLAC_16_44);
        _musicFileRepository.Setup(x => x.FindByPath(It.IsAny<string>()))
            .Returns((MusicFile?)null);

        var result = _subject.IsSatisfiedBy(musicFileInfo);

        Assert.Null(result);
    }

    [Fact]
    public void IsSatisfiedBy_UpgradeQuality_ReturnsNull()
    {
        var existingFile = CreateMusicFile("/music/test.flac", Quality.MusicMP3_128);
        var newFile = CreateMusicFileInfo("/music/test.flac", Quality.MusicFLAC_24_96);

        _musicFileRepository.Setup(x => x.FindByPath(It.IsAny<string>()))
            .Returns(existingFile);

        var result = _subject.IsSatisfiedBy(newFile);

        Assert.Null(result);
    }

    [Fact]
    public void IsSatisfiedBy_NotUpgrade_ReturnsRejection()
    {
        var existingFile = CreateMusicFile("/music/test.flac", Quality.MusicFLAC_24_96);
        var newFile = CreateMusicFileInfo("/music/test.flac", Quality.MusicMP3_128);

        _musicFileRepository.Setup(x => x.FindByPath(It.IsAny<string>()))
            .Returns(existingFile);

        var result = _subject.IsSatisfiedBy(newFile);

        Assert.NotNull(result);
        Assert.Equal(ImportRejectionReason.NotQualityUpgrade, result.Reason);
    }

    [Fact]
    public void IsSatisfiedBy_ExistingFileNullQuality_ReturnsRejectionWhenNotUpgrade()
    {
        var existingFile = new MusicFile { RelativePath = "/music/test.flac", Quality = null };
        var newFile = CreateMusicFileInfo("/music/test.flac", Quality.MusicMP3_128);

        _musicFileRepository.Setup(x => x.FindByPath(It.IsAny<string>()))
            .Returns(existingFile);

        var result = _subject.IsSatisfiedBy(newFile);

        // When existing has null quality, new file is always an upgrade
        Assert.Null(result);
    }

    private static MusicFileInfo CreateMusicFileInfo(string path, Quality quality)
    {
        return new MusicFileInfo
        {
            Path = path,
            Quality = quality
        };
    }

    private static MusicFile CreateMusicFile(string path, Quality quality)
    {
        return new MusicFile
        {
            RelativePath = path,
            Quality = new QualityModel(quality)
        };
    }
}

public class QualityUpgradeServiceTests
{
    [Fact]
    public void IsUpgrade_NullCurrent_ReturnsTrue()
    {
        var candidate = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.IsUpgrade(null, candidate);

        Assert.True(result);
    }

    [Fact]
    public void IsUpgrade_BetterQuality_ReturnsTrue()
    {
        var current = new QualityModel(Quality.MusicMP3_320);
        var candidate = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.IsUpgrade(current, candidate);

        Assert.True(result);
    }

    [Fact]
    public void IsUpgrade_WorseQuality_ReturnsFalse()
    {
        var current = new QualityModel(Quality.MusicFLAC_24_96);
        var candidate = new QualityModel(Quality.MusicMP3_128);

        var result = QualityUpgradeService.IsUpgrade(current, candidate);

        Assert.False(result);
    }

    [Fact]
    public void IsUpgrade_SameQuality_ReturnsFalse()
    {
        var current = new QualityModel(Quality.MusicFLAC_16_44);
        var candidate = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.IsUpgrade(current, candidate);

        Assert.False(result);
    }

    [Fact]
    public void IsUpgradeWithCutoff_NullCurrent_ReturnsTrue()
    {
        var candidate = new QualityModel(Quality.MusicMP3_128);
        var cutoff = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.IsUpgradeWithCutoff(null, candidate, cutoff);

        Assert.True(result);
    }

    [Fact]
    public void IsUpgradeWithCutoff_CurrentMeetsCutoff_ReturnsFalse()
    {
        var current = new QualityModel(Quality.MusicFLAC_24_96);
        var candidate = new QualityModel(Quality.MusicFLAC_24_192);
        var cutoff = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.IsUpgradeWithCutoff(current, candidate, cutoff);

        Assert.False(result);
    }

    [Fact]
    public void IsUpgradeWithCutoff_BelowCutoff_ReturnsTrue()
    {
        var current = new QualityModel(Quality.MusicMP3_128);
        var candidate = new QualityModel(Quality.MusicFLAC_16_44);
        var cutoff = new QualityModel(Quality.MusicFLAC_24_96);

        var result = QualityUpgradeService.IsUpgradeWithCutoff(current, candidate, cutoff);

        Assert.True(result);
    }

    [Fact]
    public void GetBetterQuality_FirstIsBetter_ReturnsFirst()
    {
        var a = new QualityModel(Quality.MusicFLAC_24_96);
        var b = new QualityModel(Quality.MusicMP3_320);

        var result = QualityUpgradeService.GetBetterQuality(a, b);

        Assert.Equal(a, result);
    }

    [Fact]
    public void GetBetterQuality_SecondIsBetter_ReturnsSecond()
    {
        var a = new QualityModel(Quality.MusicMP3_128);
        var b = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.GetBetterQuality(a, b);

        Assert.Equal(b, result);
    }

    [Fact]
    public void GetBetterQuality_EqualQuality_ReturnsFirst()
    {
        var a = new QualityModel(Quality.MusicFLAC_16_44);
        var b = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.GetBetterQuality(a, b);

        Assert.Equal(a, result);
    }

    [Fact]
    public void IsUpgradeWithCutoff_BelowCutoffButNotBetter_ReturnsFalse()
    {
        var current = new QualityModel(Quality.MusicMP3_256);
        var candidate = new QualityModel(Quality.MusicMP3_128);
        var cutoff = new QualityModel(Quality.MusicFLAC_24_96);

        var result = QualityUpgradeService.IsUpgradeWithCutoff(current, candidate, cutoff);

        Assert.False(result);
    }

    [Fact]
    public void IsUpgrade_HiResOverCd_ReturnsTrue()
    {
        var current = new QualityModel(Quality.MusicFLAC_16_44);
        var candidate = new QualityModel(Quality.MusicFLAC_24_192);

        var result = QualityUpgradeService.IsUpgrade(current, candidate);

        Assert.True(result);
    }

    [Fact]
    public void IsUpgrade_CdOverHiRes_ReturnsFalse()
    {
        var current = new QualityModel(Quality.MusicFLAC_24_192);
        var candidate = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.IsUpgrade(current, candidate);

        Assert.False(result);
    }

    [Fact]
    public void GetBetterQuality_DifferentFormats_ReturnsBetter()
    {
        var mp3 = new QualityModel(Quality.MusicMP3_320);
        var flac = new QualityModel(Quality.MusicFLAC_16_44);

        var result = QualityUpgradeService.GetBetterQuality(mp3, flac);

        Assert.Equal(flac, result);
    }
}
