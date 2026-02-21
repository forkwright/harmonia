// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Authentication;

namespace Mouseion.Api.Authentication;

[ApiController]
[Route("api/v3/users")]
[Authorize]
public class UsersController : ControllerBase
{
    private readonly IUserRepository _userRepository;
    private readonly IAuthenticationService _authService;
    private readonly IJwtTokenService _jwtTokenService;

    public UsersController(
        IUserRepository userRepository,
        IAuthenticationService authService,
        IJwtTokenService jwtTokenService)
    {
        _userRepository = userRepository;
        _authService = authService;
        _jwtTokenService = jwtTokenService;
    }

    [HttpGet("me")]
    public async Task<ActionResult<UserResource>> GetCurrentUser(CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        if (!userId.HasValue)
        {
            return Unauthorized();
        }

        var user = await _userRepository.FindAsync(userId.Value, ct).ConfigureAwait(false);
        if (user == null)
        {
            return NotFound();
        }

        return Ok(AuthController.ToUserResource(user));
    }

    [HttpGet]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<List<UserResource>>> GetAllUsers(CancellationToken ct = default)
    {
        var users = await _userRepository.AllAsync(ct).ConfigureAwait(false);
        return Ok(users.Select(AuthController.ToUserResource).ToList());
    }

    [HttpGet("{id:int}")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<UserResource>> GetUser(int id, CancellationToken ct = default)
    {
        var user = await _userRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (user == null)
        {
            return NotFound(new { error = $"User {id} not found" });
        }

        return Ok(AuthController.ToUserResource(user));
    }

    [HttpPost]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<UserResource>> CreateUser(
        [FromBody][Required] CreateUserApiRequest request,
        CancellationToken ct = default)
    {
        try
        {
            var user = await _authService.CreateUserAsync(new CreateUserRequest
            {
                Username = request.Username,
                Password = request.Password,
                DisplayName = request.DisplayName,
                Email = request.Email,
                Role = Enum.Parse<UserRole>(request.Role ?? "User", ignoreCase: true)
            }, ct).ConfigureAwait(false);

            return CreatedAtAction(nameof(GetUser), new { id = user.Id }, AuthController.ToUserResource(user));
        }
        catch (InvalidOperationException ex)
        {
            return Conflict(new { error = ex.Message });
        }
    }

    [HttpPut("{id:int}")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult<UserResource>> UpdateUser(
        int id,
        [FromBody][Required] UpdateUserApiRequest request,
        CancellationToken ct = default)
    {
        try
        {
            UserRole? role = null;
            if (request.Role != null)
            {
                role = Enum.Parse<UserRole>(request.Role, ignoreCase: true);
            }

            var user = await _authService.UpdateUserAsync(id, new UpdateUserRequest
            {
                DisplayName = request.DisplayName,
                Email = request.Email,
                Role = role
            }, ct).ConfigureAwait(false);

            return Ok(AuthController.ToUserResource(user));
        }
        catch (InvalidOperationException ex)
        {
            return ex.Message.Contains("not found")
                ? NotFound(new { error = ex.Message })
                : Conflict(new { error = ex.Message });
        }
    }

    [HttpDelete("{id:int}")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult> DeactivateUser(int id, CancellationToken ct = default)
    {
        var currentUserId = GetCurrentUserId();
        if (currentUserId == id)
        {
            return BadRequest(new { error = "Cannot deactivate your own account" });
        }

        try
        {
            await _authService.DeactivateUserAsync(id, ct).ConfigureAwait(false);
            await _jwtTokenService.RevokeAllTokensForUserAsync(id, ct).ConfigureAwait(false);
            return NoContent();
        }
        catch (InvalidOperationException ex)
        {
            return NotFound(new { error = ex.Message });
        }
    }

    [HttpPost("{id:int}/reset-password")]
    [Authorize(Roles = "Admin")]
    public async Task<ActionResult> ResetPassword(
        int id,
        [FromBody][Required] ResetPasswordRequest request,
        CancellationToken ct = default)
    {
        try
        {
            await _authService.ResetPasswordAsync(id, request.NewPassword, ct).ConfigureAwait(false);
            await _jwtTokenService.RevokeAllTokensForUserAsync(id, ct).ConfigureAwait(false);
            return NoContent();
        }
        catch (InvalidOperationException ex)
        {
            return NotFound(new { error = ex.Message });
        }
    }

    private int? GetCurrentUserId()
    {
        var claim = User.FindFirst(ClaimTypes.NameIdentifier);
        return claim != null ? int.Parse(claim.Value) : null;
    }
}

// API-specific request DTOs
public class CreateUserApiRequest
{
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public string? DisplayName { get; set; }
    public string? Email { get; set; }
    public string? Role { get; set; }
}

public class UpdateUserApiRequest
{
    public string? DisplayName { get; set; }
    public string? Email { get; set; }
    public string? Role { get; set; }
}

public class ResetPasswordRequest
{
    public string NewPassword { get; set; } = string.Empty;
}
