// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Mouseion.Core.MediaFiles.Import;

namespace Mouseion.Core.Tests.MediaFiles.Import;

public class MediaFileVerificationServiceTests : IDisposable
{
    private readonly MediaFileVerificationService _service;
    private readonly string _tempDir;

    public MediaFileVerificationServiceTests()
    {
        _service = new MediaFileVerificationService(
            NullLogger<MediaFileVerificationService>.Instance);

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
    public async Task VerifyFileIntegrityAsync_should_return_true_for_identical_files()
    {
        var sourceFile = Path.Combine(_tempDir, "source.txt");
        var destFile = Path.Combine(_tempDir, "dest.txt");

        var content = "Test file content for verification";
        await File.WriteAllTextAsync(sourceFile, content);
        await File.WriteAllTextAsync(destFile, content);

        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile);

        Assert.True(result);
    }

    [Fact]
    public async Task VerifyFileIntegrityAsync_should_return_false_for_different_sizes()
    {
        var sourceFile = Path.Combine(_tempDir, "source.txt");
        var destFile = Path.Combine(_tempDir, "dest.txt");

        await File.WriteAllTextAsync(sourceFile, "Short content");
        await File.WriteAllTextAsync(destFile, "Much longer content that differs");

        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile);

        Assert.False(result);
    }

    [Fact]
    public async Task VerifyFileIntegrityAsync_should_return_false_for_different_content()
    {
        var sourceFile = Path.Combine(_tempDir, "source.txt");
        var destFile = Path.Combine(_tempDir, "dest.txt");

        // Same length, different content
        await File.WriteAllTextAsync(sourceFile, "AAAA");
        await File.WriteAllTextAsync(destFile, "BBBB");

        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile);

        Assert.False(result);
    }

    [Fact]
    public async Task VerifyFileIntegrityAsync_should_return_false_when_source_missing()
    {
        var sourceFile = Path.Combine(_tempDir, "nonexistent.txt");
        var destFile = Path.Combine(_tempDir, "dest.txt");

        await File.WriteAllTextAsync(destFile, "Content");

        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile);

        Assert.False(result);
    }

    [Fact]
    public async Task VerifyFileIntegrityAsync_should_return_false_when_dest_missing()
    {
        var sourceFile = Path.Combine(_tempDir, "source.txt");
        var destFile = Path.Combine(_tempDir, "nonexistent.txt");

        await File.WriteAllTextAsync(sourceFile, "Content");

        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile);

        Assert.False(result);
    }

    [Fact]
    public async Task VerifyFileIntegrityAsync_should_skip_checksum_when_disabled()
    {
        var sourceFile = Path.Combine(_tempDir, "source.txt");
        var destFile = Path.Combine(_tempDir, "dest.txt");

        var content = "Test content";
        await File.WriteAllTextAsync(sourceFile, content);
        await File.WriteAllTextAsync(destFile, content);

        // Should only verify size, not checksum
        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile, verifyChecksum: false);

        Assert.True(result);
    }

    [Fact]
    public void CalculateChecksum_should_return_consistent_hash()
    {
        var testFile = Path.Combine(_tempDir, "test.txt");
        File.WriteAllText(testFile, "Hello, World!");

        var checksum1 = _service.CalculateChecksum(testFile);
        var checksum2 = _service.CalculateChecksum(testFile);

        Assert.Equal(checksum1, checksum2);
        Assert.NotEmpty(checksum1);
        Assert.Equal(32, checksum1.Length); // MD5 is 32 hex characters
    }

    [Fact]
    public void CalculateChecksum_should_return_different_hash_for_different_content()
    {
        var file1 = Path.Combine(_tempDir, "file1.txt");
        var file2 = Path.Combine(_tempDir, "file2.txt");

        File.WriteAllText(file1, "Content A");
        File.WriteAllText(file2, "Content B");

        var checksum1 = _service.CalculateChecksum(file1);
        var checksum2 = _service.CalculateChecksum(file2);

        Assert.NotEqual(checksum1, checksum2);
    }

    [Fact]
    public void CalculateChecksum_should_return_same_hash_for_same_content()
    {
        var file1 = Path.Combine(_tempDir, "file1.txt");
        var file2 = Path.Combine(_tempDir, "file2.txt");

        var content = "Same content in both files";
        File.WriteAllText(file1, content);
        File.WriteAllText(file2, content);

        var checksum1 = _service.CalculateChecksum(file1);
        var checksum2 = _service.CalculateChecksum(file2);

        Assert.Equal(checksum1, checksum2);
    }

    [Fact]
    public void CalculateChecksum_should_throw_for_nonexistent_file()
    {
        var nonexistentFile = Path.Combine(_tempDir, "nonexistent.txt");

        Assert.Throws<FileNotFoundException>(() => _service.CalculateChecksum(nonexistentFile));
    }

    [Fact]
    public async Task VerifyFileIntegrityAsync_should_handle_large_files()
    {
        var sourceFile = Path.Combine(_tempDir, "large_source.bin");
        var destFile = Path.Combine(_tempDir, "large_dest.bin");

        // Create 1MB files
        var largeContent = new byte[1024 * 1024];
        new Random(42).NextBytes(largeContent);

        await File.WriteAllBytesAsync(sourceFile, largeContent);
        await File.WriteAllBytesAsync(destFile, largeContent);

        var result = await _service.VerifyFileIntegrityAsync(sourceFile, destFile);

        Assert.True(result);
    }
}
