// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;
using System.Security;
using FluentValidation;

namespace Mouseion.Api.RootFolders;

public class RootFolderResourceValidator : AbstractValidator<RootFolderResource>
{
    public RootFolderResourceValidator()
    {
        RuleFor(x => x.Path)
            .NotEmpty().WithMessage("Path is required")
            .MaximumLength(1000).WithMessage("Path must not exceed 1000 characters")
            .Must(BeAValidPath).WithMessage("Path must be a valid absolute path");

        RuleFor(x => x.MediaType)
            .InclusiveBetween(0, 6).WithMessage("Media type must be a valid value (0-6)");

        RuleFor(x => x.FreeSpace)
            .GreaterThanOrEqualTo(0).When(x => x.FreeSpace.HasValue)
            .WithMessage("Free space must be 0 or greater");

        RuleFor(x => x.TotalSpace)
            .GreaterThan(0).When(x => x.TotalSpace.HasValue)
            .WithMessage("Total space must be greater than 0");

        RuleFor(x => x)
            .Must(x => !x.FreeSpace.HasValue || !x.TotalSpace.HasValue || x.FreeSpace <= x.TotalSpace)
            .WithMessage("Free space cannot exceed total space");
    }

    private bool BeAValidPath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return false;

        try
        {
            // Check if the path is rooted (absolute)
            if (!Path.IsPathRooted(path))
                return false;

            // Check for path traversal attempts
            var fullPath = Path.GetFullPath(path);
            if (!path.Equals(fullPath, StringComparison.OrdinalIgnoreCase))
                return false;

            // Basic invalid characters check
            var invalidChars = Path.GetInvalidPathChars();
            if (path.Any(c => invalidChars.Contains(c)))
                return false;

            return true;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (SecurityException)
        {
            return false;
        }
        catch (NotSupportedException)
        {
            return false;
        }
        catch (PathTooLongException)
        {
            return false;
        }
    }
}
