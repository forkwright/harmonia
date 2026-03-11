// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using Serilog;

namespace Mouseion.Core.Authentication;

public interface IAuthenticationService
{
    Task<User?> ValidateCredentialsAsync(string username, string password, CancellationToken ct = default);
    Task<User> CreateUserAsync(CreateUserRequest request, CancellationToken ct = default);
    Task<User> UpdateUserAsync(int userId, UpdateUserRequest request, CancellationToken ct = default);
    Task DeactivateUserAsync(int userId, CancellationToken ct = default);
    Task ChangePasswordAsync(int userId, string currentPassword, string newPassword, CancellationToken ct = default);
    Task ResetPasswordAsync(int userId, string newPassword, CancellationToken ct = default);
    string HashPassword(string password);
    bool VerifyPassword(string password, string hash);
}

public class AuthenticationService : IAuthenticationService
{
    private readonly IUserRepository _userRepository;
    private static readonly ILogger Logger = Log.ForContext<AuthenticationService>();

    // BCrypt-style work factor using PBKDF2 (no external dependency needed)
    private const int SaltSize = 16;
    private const int HashSize = 32;
    private const int Iterations = 100_000;
    private static readonly HashAlgorithmName Algorithm = HashAlgorithmName.SHA512;

    public AuthenticationService(IUserRepository userRepository)
    {
        _userRepository = userRepository;
    }

    public async Task<User?> ValidateCredentialsAsync(string username, string password, CancellationToken ct = default)
    {
        var user = await _userRepository.GetByUsernameAsync(username, ct).ConfigureAwait(false);

        if (user == null || !user.IsActive)
        {
            Logger.Warning("Login attempt for unknown or inactive user: {Username}", username);
            return null;
        }

        if (user.AuthenticationMethod != "local")
        {
            Logger.Warning("Local login attempt for non-local user: {Username} (method: {Method})",
                username, user.AuthenticationMethod);
            return null;
        }

        if (!VerifyPassword(password, user.PasswordHash))
        {
            Logger.Warning("Invalid password for user: {Username}", username);
            return null;
        }

        await _userRepository.UpdateLastLoginAsync(user.Id, ct).ConfigureAwait(false);
        Logger.Information("User authenticated: {Username}", username);

        return user;
    }

    public async Task<User> CreateUserAsync(CreateUserRequest request, CancellationToken ct = default)
    {
        if (await _userRepository.UsernameExistsAsync(request.Username, ct).ConfigureAwait(false))
        {
            throw new InvalidOperationException($"Username '{request.Username}' is already taken");
        }

        if (!string.IsNullOrEmpty(request.Email) &&
            await _userRepository.EmailExistsAsync(request.Email, ct).ConfigureAwait(false))
        {
            throw new InvalidOperationException($"Email '{request.Email}' is already registered");
        }

        var user = new User
        {
            Username = request.Username,
            DisplayName = request.DisplayName ?? request.Username,
            Email = request.Email ?? string.Empty,
            Role = request.Role,
            AuthenticationMethod = "local",
            PasswordHash = HashPassword(request.Password),
            IsActive = true,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        var created = await _userRepository.InsertAsync(user, ct).ConfigureAwait(false);
        Logger.Information("User created: {Username} (role: {Role})", created.Username, created.Role);

        return created;
    }

    public async Task<User> UpdateUserAsync(int userId, UpdateUserRequest request, CancellationToken ct = default)
    {
        var user = await _userRepository.GetAsync(userId, ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"User {userId} not found");

        if (request.DisplayName != null)
            user.DisplayName = request.DisplayName;

        if (request.Email != null)
        {
            if (await _userRepository.EmailExistsAsync(request.Email, ct).ConfigureAwait(false) &&
                !string.Equals(user.Email, request.Email, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException($"Email '{request.Email}' is already registered");
            }
            user.Email = request.Email;
        }

        if (request.Role.HasValue)
            user.Role = request.Role.Value;

        user.UpdatedAt = DateTime.UtcNow;

        return await _userRepository.UpdateAsync(user, ct).ConfigureAwait(false);
    }

    public async Task DeactivateUserAsync(int userId, CancellationToken ct = default)
    {
        var user = await _userRepository.GetAsync(userId, ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"User {userId} not found");

        user.IsActive = false;
        user.UpdatedAt = DateTime.UtcNow;

        await _userRepository.UpdateAsync(user, ct).ConfigureAwait(false);
        Logger.Information("User deactivated: {Username}", user.Username);
    }

    public async Task ChangePasswordAsync(int userId, string currentPassword, string newPassword, CancellationToken ct = default)
    {
        var user = await _userRepository.GetAsync(userId, ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"User {userId} not found");

        if (!VerifyPassword(currentPassword, user.PasswordHash))
        {
            throw new UnauthorizedAccessException("Current password is incorrect");
        }

        user.PasswordHash = HashPassword(newPassword);
        user.UpdatedAt = DateTime.UtcNow;

        await _userRepository.UpdateAsync(user, ct).ConfigureAwait(false);
        Logger.Information("Password changed for user: {Username}", user.Username);
    }

    public async Task ResetPasswordAsync(int userId, string newPassword, CancellationToken ct = default)
    {
        var user = await _userRepository.GetAsync(userId, ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"User {userId} not found");

        user.PasswordHash = HashPassword(newPassword);
        user.UpdatedAt = DateTime.UtcNow;

        await _userRepository.UpdateAsync(user, ct).ConfigureAwait(false);
        Logger.Information("Password reset for user: {Username}", user.Username);
    }

    public string HashPassword(string password)
    {
        var salt = RandomNumberGenerator.GetBytes(SaltSize);
        var hash = Rfc2898DeriveBytes.Pbkdf2(password, salt, Iterations, Algorithm, HashSize);

        // Format: iterations.algorithm.salt.hash (all base64)
        return $"{Iterations}.{Algorithm.Name}.{Convert.ToBase64String(salt)}.{Convert.ToBase64String(hash)}";
    }

    public bool VerifyPassword(string password, string storedHash)
    {
        try
        {
            var parts = storedHash.Split('.');
            if (parts.Length != 4) return false;

            var iterations = int.Parse(parts[0]);
            var algorithm = new HashAlgorithmName(parts[1]);
            var salt = Convert.FromBase64String(parts[2]);
            var hash = Convert.FromBase64String(parts[3]);

            var computedHash = Rfc2898DeriveBytes.Pbkdf2(password, salt, iterations, algorithm, hash.Length);

            return CryptographicOperations.FixedTimeEquals(computedHash, hash);
        }
        catch
        {
            return false;
        }
    }
}

public class CreateUserRequest
{
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public string? DisplayName { get; set; }
    public string? Email { get; set; }
    public UserRole Role { get; set; } = UserRole.User;
}

public class UpdateUserRequest
{
    public string? DisplayName { get; set; }
    public string? Email { get; set; }
    public UserRole? Role { get; set; }
}
