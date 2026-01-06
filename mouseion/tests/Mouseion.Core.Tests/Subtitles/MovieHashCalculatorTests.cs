// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Xunit;
using Mouseion.Core.Subtitles;

namespace Mouseion.Core.Tests.Subtitles;

public class MovieHashCalculatorTests
{
    [Fact]
    public void ComputeHash_ValidFile_ShouldReturnHexHash()
    {
        var tempFile = Path.GetTempFileName();
        try
        {
            var random = new Random(42);
            var data = new byte[128 * 1024];
            random.NextBytes(data);
            File.WriteAllBytes(tempFile, data);

            var hash = MovieHashCalculator.ComputeHash(tempFile);

            Assert.NotNull(hash);
            Assert.Equal(16, hash.Length);
            Assert.Matches("^[0-9a-f]{16}$", hash);
        }
        finally
        {
            File.Delete(tempFile);
        }
    }

    [Fact]
    public void ComputeHash_FileTooSmall_ShouldThrowArgumentException()
    {
        var tempFile = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(tempFile, new byte[1024]);

            var ex = Assert.Throws<ArgumentException>(() => MovieHashCalculator.ComputeHash(tempFile));
            Assert.Contains("too small", ex.Message);
        }
        finally
        {
            File.Delete(tempFile);
        }
    }

    [Fact]
    public void ComputeHash_FileNotFound_ShouldThrowFileNotFoundException()
    {
        var nonExistentFile = Path.Combine(Path.GetTempPath(), "non_existent_movie.mkv");

        Assert.Throws<FileNotFoundException>(() => MovieHashCalculator.ComputeHash(nonExistentFile));
    }

    [Fact]
    public void ComputeHash_SameFile_ShouldReturnSameHash()
    {
        var tempFile = Path.GetTempFileName();
        try
        {
            var random = new Random(123);
            var data = new byte[256 * 1024];
            random.NextBytes(data);
            File.WriteAllBytes(tempFile, data);

            var hash1 = MovieHashCalculator.ComputeHash(tempFile);
            var hash2 = MovieHashCalculator.ComputeHash(tempFile);

            Assert.Equal(hash1, hash2);
        }
        finally
        {
            File.Delete(tempFile);
        }
    }
}
