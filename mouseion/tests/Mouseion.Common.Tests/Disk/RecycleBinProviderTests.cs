// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Disk;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Tests.Disk;

public class RecycleBinProviderTests : IDisposable
{
    private readonly RecycleBinProvider _provider;
    private readonly string _tempDir;

    public RecycleBinProviderTests()
    {
        _provider = new RecycleBinProvider();
        _tempDir = Path.Combine(Path.GetTempPath(), $"mouseion_recycle_test_{Guid.NewGuid()}");
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
    public void IsAvailable_should_return_true_on_supported_platforms()
    {
        var isAvailable = _provider.IsAvailable;

        if (OsInfo.IsWindows || OsInfo.IsLinux || OsInfo.IsOsx)
        {
            Assert.True(isAvailable);
        }
    }

    [Fact]
    public void DeleteFile_should_return_false_when_file_does_not_exist()
    {
        var result = _provider.DeleteFile("/nonexistent/file.txt");

        Assert.False(result);
    }

    [Fact]
    public void DeleteFolder_should_return_false_when_folder_does_not_exist()
    {
        var result = _provider.DeleteFolder("/nonexistent/folder");

        Assert.False(result);
    }

    [Fact]
    public void GetRecycleBinPath_should_return_path_on_supported_platforms()
    {
        var path = _provider.GetRecycleBinPath();

        if (OsInfo.IsWindows)
        {
            Assert.Equal("$Recycle.Bin", path);
        }
        else if (OsInfo.IsLinux || OsInfo.IsOsx)
        {
            if (path != null)
            {
                Assert.Contains(".local", path);
                Assert.Contains("Trash", path);
            }
        }
    }

    [Fact]
    public void DeleteFile_should_successfully_delete_existing_file()
    {
        var testFile = Path.Combine(_tempDir, "test_file.txt");
        File.WriteAllText(testFile, "test content");

        var result = _provider.DeleteFile(testFile);

        if (_provider.IsAvailable)
        {
            Assert.True(result);
            Assert.False(File.Exists(testFile));
        }
        else
        {
            Assert.False(result);
        }
    }

    [Fact]
    public void DeleteFile_with_special_characters_should_be_handled()
    {
        var testFile = Path.Combine(_tempDir, "file with spaces & special!.txt");
        File.WriteAllText(testFile, "test content");

        var result = _provider.DeleteFile(testFile);

        if (_provider.IsAvailable)
        {
            Assert.True(result);
            Assert.False(File.Exists(testFile));
        }
        else
        {
            Assert.False(result);
        }
    }

    [Fact]
    public void DeleteFile_with_readonly_attribute_should_succeed()
    {
        var testFile = Path.Combine(_tempDir, "readonly_file.txt");
        File.WriteAllText(testFile, "test content");

        if (OsInfo.IsWindows)
        {
            File.SetAttributes(testFile, FileAttributes.ReadOnly);
        }
        else
        {
            File.SetUnixFileMode(testFile, UnixFileMode.UserRead);
        }

        var result = _provider.DeleteFile(testFile);

        if (_provider.IsAvailable)
        {
            Assert.True(result);
            Assert.False(File.Exists(testFile));
        }
        else
        {
            Assert.False(result);
        }
    }

    [Fact]
    public void Multiple_sequential_deletes_should_succeed()
    {
        var file1 = Path.Combine(_tempDir, "file1.txt");
        var file2 = Path.Combine(_tempDir, "file2.txt");
        var file3 = Path.Combine(_tempDir, "file3.txt");

        File.WriteAllText(file1, "content1");
        File.WriteAllText(file2, "content2");
        File.WriteAllText(file3, "content3");

        var result1 = _provider.DeleteFile(file1);
        var result2 = _provider.DeleteFile(file2);
        var result3 = _provider.DeleteFile(file3);

        if (_provider.IsAvailable)
        {
            Assert.True(result1);
            Assert.True(result2);
            Assert.True(result3);
            Assert.False(File.Exists(file1));
            Assert.False(File.Exists(file2));
            Assert.False(File.Exists(file3));
        }
    }

    [Fact]
    public void DeleteFile_with_very_long_filename_should_be_handled()
    {
        var longName = new string('a', 200) + ".txt";
        var testFile = Path.Combine(_tempDir, longName);
        File.WriteAllText(testFile, "test content");

        var result = _provider.DeleteFile(testFile);

        if (_provider.IsAvailable)
        {
            Assert.True(result);
            Assert.False(File.Exists(testFile));
        }
    }
}
