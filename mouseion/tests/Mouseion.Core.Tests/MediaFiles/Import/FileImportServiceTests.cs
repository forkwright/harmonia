// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Common.Disk;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.Tests.MediaFiles.Import;

public class FileImportServiceTests : IDisposable
{
    private readonly Mock<IImportStrategySelector> _strategySelectorMock;
    private readonly Mock<IDiskTransferService> _diskTransferServiceMock;
    private readonly Mock<IMediaFileVerificationService> _verificationServiceMock;
    private readonly Mock<IRecycleBinProvider> _recycleBinMock;
    private readonly FileImportService _service;
    private readonly string _tempDir;

    public FileImportServiceTests()
    {
        _strategySelectorMock = new Mock<IImportStrategySelector>();
        _diskTransferServiceMock = new Mock<IDiskTransferService>();
        _verificationServiceMock = new Mock<IMediaFileVerificationService>();
        _recycleBinMock = new Mock<IRecycleBinProvider>();

        _service = new FileImportService(
            _strategySelectorMock.Object,
            _diskTransferServiceMock.Object,
            _verificationServiceMock.Object,
            _recycleBinMock.Object,
            NullLogger<FileImportService>.Instance);

        _tempDir = Path.Combine(Path.GetTempPath(), $"mouseion_test_{Guid.NewGuid()}");
        Directory.CreateDirectory(_tempDir);
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDir))
        {
            Directory.Delete(_tempDir, recursive: true);
        }
    }

    [Fact]
    public async Task ImportFileAsync_should_return_failure_when_source_does_not_exist()
    {
        var source = Path.Combine(_tempDir, "nonexistent.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        var result = await _service.ImportFileAsync(source, dest);

        Assert.False(result.IsSuccess);
        Assert.Contains("does not exist", result.ErrorMessage);
    }

    [Fact]
    public async Task ImportFileAsync_should_select_strategy_and_transfer_file()
    {
        var source = Path.Combine(_tempDir, "source.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        File.WriteAllText(source, "test content");

        _strategySelectorMock
            .Setup(x => x.SelectStrategy(source, dest, null))
            .Returns(FileStrategy.Hardlink);

        _diskTransferServiceMock
            .Setup(x => x.TransferFile(source, dest, It.IsAny<TransferMode>(), false))
            .Returns(TransferMode.HardLink)
            .Callback(() => File.WriteAllText(dest, "test content"));

        _verificationServiceMock
            .Setup(x => x.VerifyFileIntegrityAsync(source, dest, true))
            .ReturnsAsync(true);

        var result = await _service.ImportFileAsync(source, dest);

        Assert.True(result.IsSuccess);
        Assert.Equal(dest, result.DestinationPath);
        Assert.Equal(FileStrategy.Hardlink, result.RequestedStrategy);
        Assert.Equal(TransferMode.HardLink, result.ActualMode);
    }

    [Fact]
    public async Task ImportFileAsync_should_respect_preferred_strategy()
    {
        var source = Path.Combine(_tempDir, "source.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        File.WriteAllText(source, "test content");

        _strategySelectorMock
            .Setup(x => x.SelectStrategy(source, dest, FileStrategy.Copy))
            .Returns(FileStrategy.Copy);

        _diskTransferServiceMock
            .Setup(x => x.TransferFile(source, dest, TransferMode.Copy, false))
            .Returns(TransferMode.Copy)
            .Callback(() => File.WriteAllText(dest, "test content"));

        _verificationServiceMock
            .Setup(x => x.VerifyFileIntegrityAsync(source, dest, true))
            .ReturnsAsync(true);

        var result = await _service.ImportFileAsync(source, dest, FileStrategy.Copy);

        Assert.True(result.IsSuccess);
        _strategySelectorMock.Verify(x => x.SelectStrategy(source, dest, FileStrategy.Copy), Times.Once);
    }

    [Fact]
    public async Task ImportFileAsync_should_rollback_on_verification_failure()
    {
        var source = Path.Combine(_tempDir, "source.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        File.WriteAllText(source, "test content");

        _strategySelectorMock
            .Setup(x => x.SelectStrategy(source, dest, null))
            .Returns(FileStrategy.Copy);

        _diskTransferServiceMock
            .Setup(x => x.TransferFile(source, dest, It.IsAny<TransferMode>(), false))
            .Returns(TransferMode.Copy)
            .Callback(() => File.WriteAllText(dest, "corrupted"));

        _verificationServiceMock
            .Setup(x => x.VerifyFileIntegrityAsync(source, dest, true))
            .ReturnsAsync(false);

        _recycleBinMock
            .Setup(x => x.DeleteFile(dest))
            .Returns(true);

        var result = await _service.ImportFileAsync(source, dest);

        Assert.False(result.IsSuccess);
        Assert.Contains("verification failed", result.ErrorMessage);
        _recycleBinMock.Verify(x => x.DeleteFile(dest), Times.Once);
    }

    [Fact]
    public async Task ImportFileAsync_should_skip_checksum_when_disabled()
    {
        var source = Path.Combine(_tempDir, "source.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        File.WriteAllText(source, "test content");

        _strategySelectorMock
            .Setup(x => x.SelectStrategy(source, dest, null))
            .Returns(FileStrategy.Copy);

        _diskTransferServiceMock
            .Setup(x => x.TransferFile(source, dest, It.IsAny<TransferMode>(), false))
            .Returns(TransferMode.Copy)
            .Callback(() => File.WriteAllText(dest, "test content"));

        _verificationServiceMock
            .Setup(x => x.VerifyFileIntegrityAsync(source, dest, false))
            .ReturnsAsync(true);

        var result = await _service.ImportFileAsync(source, dest, verifyChecksum: false);

        Assert.True(result.IsSuccess);
        _verificationServiceMock.Verify(x => x.VerifyFileIntegrityAsync(source, dest, false), Times.Once);
    }

    [Fact]
    public async Task ImportFileAsync_should_cleanup_on_exception()
    {
        var source = Path.Combine(_tempDir, "source.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        File.WriteAllText(source, "test content");

        _strategySelectorMock
            .Setup(x => x.SelectStrategy(source, dest, null))
            .Returns(FileStrategy.Copy);

        _diskTransferServiceMock
            .Setup(x => x.TransferFile(source, dest, It.IsAny<TransferMode>(), false))
            .Callback(() => File.WriteAllText(dest, "partial"))
            .Throws(new IOException("Disk full"));

        _recycleBinMock
            .Setup(x => x.DeleteFile(dest))
            .Returns(true);

        var result = await _service.ImportFileAsync(source, dest);

        Assert.False(result.IsSuccess);
        Assert.Contains("Disk full", result.ErrorMessage);
        _recycleBinMock.Verify(x => x.DeleteFile(dest), Times.Once);
    }

    [Fact]
    public async Task ImportFileAsync_should_fallback_to_permanent_delete_if_recycle_fails()
    {
        var source = Path.Combine(_tempDir, "source.mkv");
        var dest = Path.Combine(_tempDir, "dest.mkv");

        File.WriteAllText(source, "test content");

        _strategySelectorMock
            .Setup(x => x.SelectStrategy(source, dest, null))
            .Returns(FileStrategy.Copy);

        _diskTransferServiceMock
            .Setup(x => x.TransferFile(source, dest, It.IsAny<TransferMode>(), false))
            .Returns(TransferMode.Copy)
            .Callback(() => File.WriteAllText(dest, "corrupted"));

        _verificationServiceMock
            .Setup(x => x.VerifyFileIntegrityAsync(source, dest, true))
            .ReturnsAsync(false);

        _recycleBinMock
            .Setup(x => x.DeleteFile(dest))
            .Returns(false);

        var result = await _service.ImportFileAsync(source, dest);

        Assert.False(result.IsSuccess);
        _recycleBinMock.Verify(x => x.DeleteFile(dest), Times.Once);
        // File should be deleted even if recycle bin failed
        Assert.False(File.Exists(dest));
    }

    [Fact]
    public void ImportResult_Success_should_set_properties_correctly()
    {
        var result = ImportResult.Success("/path/to/file.mkv", FileStrategy.Hardlink, TransferMode.HardLink);

        Assert.True(result.IsSuccess);
        Assert.Equal("/path/to/file.mkv", result.DestinationPath);
        Assert.Equal(FileStrategy.Hardlink, result.RequestedStrategy);
        Assert.Equal(TransferMode.HardLink, result.ActualMode);
        Assert.Null(result.ErrorMessage);
    }

    [Fact]
    public void ImportResult_Failure_should_set_error_message()
    {
        var result = ImportResult.Failure("Something went wrong");

        Assert.False(result.IsSuccess);
        Assert.Equal("Something went wrong", result.ErrorMessage);
        Assert.Null(result.DestinationPath);
    }
}
