// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using Mouseion.Common.EnvironmentInfo;
using Serilog;

namespace Mouseion.Common.Disk;

public class RecycleBinProvider : IRecycleBinProvider
{
    private static readonly ILogger Logger = Log.ForContext<RecycleBinProvider>();

    public bool IsAvailable => OsInfo.IsWindows || OsInfo.IsLinux || OsInfo.IsOsx;

    public bool DeleteFile(string path)
    {
        if (!File.Exists(path))
        {
            Logger.Warning("Cannot delete file {Path}: file does not exist", path);
            return false;
        }

        try
        {
            if (OsInfo.IsWindows)
            {
                return DeleteFileWindows(path);
            }
            else if (OsInfo.IsLinux || OsInfo.IsOsx)
            {
                return DeleteFileUnix(path, isDirectory: false);
            }

            Logger.Warning("Recycle bin not supported on this platform");
            return false;
        }
        catch (Exception ex)
        {
            Logger.Error(ex, "Failed to delete file {Path} to recycle bin", path);
            return false;
        }
    }

    public bool DeleteFolder(string path)
    {
        if (!Directory.Exists(path))
        {
            Logger.Warning("Cannot delete folder {Path}: folder does not exist", path);
            return false;
        }

        try
        {
            if (OsInfo.IsWindows)
            {
                return DeleteFileWindows(path);
            }
            else if (OsInfo.IsLinux || OsInfo.IsOsx)
            {
                return DeleteFileUnix(path, isDirectory: true);
            }

            Logger.Warning("Recycle bin not supported on this platform");
            return false;
        }
        catch (Exception ex)
        {
            Logger.Error(ex, "Failed to delete folder {Path} to recycle bin", path);
            return false;
        }
    }

    public string? GetRecycleBinPath()
    {
        try
        {
            if (OsInfo.IsWindows)
            {
                // Windows recycle bin is virtual, no single path
                return "$Recycle.Bin";
            }
            else if (OsInfo.IsLinux || OsInfo.IsOsx)
            {
                var homeDir = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
                var trashPath = Path.Combine(homeDir, ".local", "share", "Trash");

                if (Directory.Exists(trashPath))
                {
                    return trashPath;
                }
            }

            return null;
        }
        catch (Exception ex)
        {
            Logger.Debug(ex, "Failed to get recycle bin path");
            return null;
        }
    }

    [ExcludeFromCodeCoverage] // Windows-only P/Invoke - cannot test on Linux CI
    private bool DeleteFileWindows(string path)
    {
        try
        {
            var fileOp = new SHFILEOPSTRUCT
            {
                wFunc = FileOperationType.FO_DELETE,
                pFrom = path + '\0' + '\0',
                fFlags = FileOperationFlags.FOF_ALLOWUNDO | FileOperationFlags.FOF_NOCONFIRMATION | FileOperationFlags.FOF_SILENT
            };

            var result = SHFileOperation(ref fileOp);

            if (result != 0)
            {
                Logger.Warning("SHFileOperation returned error code {Code} for {Path}", result, path);
                return false;
            }

            return true;
        }
        catch (Exception ex)
        {
            Logger.Error(ex, "Windows recycle bin operation failed for {Path}", path);
            return false;
        }
    }

    private bool DeleteFileUnix(string path, bool isDirectory)
    {
        try
        {
            // Follow freedesktop.org Trash specification
            var homeDir = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            var trashDir = Path.Combine(homeDir, ".local", "share", "Trash");
            var filesDir = Path.Combine(trashDir, "files");
            var infoDir = Path.Combine(trashDir, "info");

            // Ensure trash directories exist
            Directory.CreateDirectory(filesDir);
            Directory.CreateDirectory(infoDir);

            // Generate unique name
            var fileName = Path.GetFileName(path);
            var destPath = Path.Combine(filesDir, fileName);
            var counter = 1;

            while (File.Exists(destPath) || Directory.Exists(destPath))
            {
                destPath = Path.Combine(filesDir, $"{Path.GetFileNameWithoutExtension(fileName)}_{counter}{Path.GetExtension(fileName)}");
                counter++;
            }

            // Move file/folder to trash
            if (isDirectory)
            {
                Directory.Move(path, destPath);
            }
            else
            {
                File.Move(path, destPath);
            }

            // Create .trashinfo file
            var infoFileName = Path.GetFileName(destPath) + ".trashinfo";
            var infoPath = Path.Combine(infoDir, infoFileName);
            var deletionDate = DateTime.Now.ToString("yyyy-MM-ddTHH:mm:ss");

            var infoContent = $"[Trash Info]\nPath={path}\nDeletionDate={deletionDate}\n";
            File.WriteAllText(infoPath, infoContent);

            Logger.Debug("Moved {Path} to trash at {DestPath}", path, destPath);
            return true;
        }
        catch (Exception ex)
        {
            Logger.Error(ex, "Unix trash operation failed for {Path}", path);
            return false;
        }
    }

    #region Windows P/Invoke

    [DllImport("shell32.dll", CharSet = CharSet.Auto)]
    private static extern int SHFileOperation(ref SHFILEOPSTRUCT FileOp);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
    private struct SHFILEOPSTRUCT
    {
        public IntPtr hwnd;

        [MarshalAs(UnmanagedType.U4)]
        public FileOperationType wFunc;

        public string pFrom;
        public string? pTo;
        public FileOperationFlags fFlags;

        [MarshalAs(UnmanagedType.Bool)]
        public bool fAnyOperationsAborted;

        public IntPtr hNameMappings;
        public string? lpszProgressTitle;
    }

    [Flags]
    private enum FileOperationFlags : ushort
    {
        FOF_SILENT = 0x0004,
        FOF_NOCONFIRMATION = 0x0010,
        FOF_ALLOWUNDO = 0x0040,
        FOF_SIMPLEPROGRESS = 0x0100,
        FOF_NOERRORUI = 0x0400,
        FOF_WANTNUKEWARNING = 0x4000,
    }

    private enum FileOperationType : uint
    {
        FO_MOVE = 0x0001,
        FO_COPY = 0x0002,
        FO_DELETE = 0x0003,
        FO_RENAME = 0x0004,
    }

    #endregion
}
