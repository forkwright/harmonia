// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Claims;
using FluentAssertions;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Authentication;
using Mouseion.Core.Authentication;
using Xunit;

namespace Mouseion.Api.Tests.Authentication;

public class UsersControllerTests
{
    private readonly Mock<IUserRepository> _userRepo;
    private readonly Mock<IAuthenticationService> _authService;
    private readonly Mock<IJwtTokenService> _jwtService;
    private readonly UsersController _sut;

    public UsersControllerTests()
    {
        _userRepo = new Mock<IUserRepository>();
        _authService = new Mock<IAuthenticationService>();
        _jwtService = new Mock<IJwtTokenService>();
        _sut = new UsersController(_userRepo.Object, _authService.Object, _jwtService.Object);
    }

    private void SetCurrentUser(int userId, string role = "Admin")
    {
        var claims = new List<Claim>
        {
            new(ClaimTypes.NameIdentifier, userId.ToString()),
            new(ClaimTypes.Role, role)
        };
        var identity = new ClaimsIdentity(claims, "test");
        _sut.ControllerContext = new ControllerContext
        {
            HttpContext = new DefaultHttpContext { User = new ClaimsPrincipal(identity) }
        };
    }

    [Fact]
    public async Task GetCurrentUser_Authenticated_ReturnsUser()
    {
        SetCurrentUser(1);

        var user = new User
        {
            Id = 1, Username = "admin", DisplayName = "Admin",
            Email = "admin@test.com", Role = UserRole.Admin, IsActive = true
        };
        _userRepo.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>())).ReturnsAsync(user);

        var result = await _sut.GetCurrentUser();

        var ok = result.Result.Should().BeOfType<OkObjectResult>().Subject;
        var resource = ok.Value.Should().BeOfType<UserResource>().Subject;
        resource.Username.Should().Be("admin");
        resource.Role.Should().Be("Admin");
    }

    [Fact]
    public async Task GetAllUsers_ReturnsAllUsers()
    {
        SetCurrentUser(1);

        var users = new List<User>
        {
            new() { Id = 1, Username = "admin", DisplayName = "Admin", Role = UserRole.Admin },
            new() { Id = 2, Username = "user1", DisplayName = "User One", Role = UserRole.User }
        };
        _userRepo.Setup(r => r.AllAsync(It.IsAny<CancellationToken>())).ReturnsAsync(users);

        var result = await _sut.GetAllUsers();

        var ok = result.Result.Should().BeOfType<OkObjectResult>().Subject;
        var list = ok.Value.Should().BeAssignableTo<List<UserResource>>().Subject;
        list.Should().HaveCount(2);
    }

    [Fact]
    public async Task CreateUser_Valid_ReturnsCreated()
    {
        SetCurrentUser(1);

        var created = new User
        {
            Id = 2, Username = "newuser", DisplayName = "New User",
            Role = UserRole.User, IsActive = true
        };
        _authService.Setup(s => s.CreateUserAsync(It.IsAny<CreateUserRequest>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(created);

        var result = await _sut.CreateUser(new CreateUserApiRequest
        {
            Username = "newuser", Password = "password123", DisplayName = "New User"
        });

        result.Result.Should().BeOfType<CreatedAtActionResult>();
    }

    [Fact]
    public async Task CreateUser_Duplicate_ReturnsConflict()
    {
        SetCurrentUser(1);

        _authService.Setup(s => s.CreateUserAsync(It.IsAny<CreateUserRequest>(), It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("Username 'taken' is already taken"));

        var result = await _sut.CreateUser(new CreateUserApiRequest
        {
            Username = "taken", Password = "password123"
        });

        result.Result.Should().BeOfType<ConflictObjectResult>();
    }

    [Fact]
    public async Task DeactivateUser_Self_ReturnsBadRequest()
    {
        SetCurrentUser(1);

        var result = await _sut.DeactivateUser(1);

        result.Should().BeOfType<BadRequestObjectResult>();
    }

    [Fact]
    public async Task DeactivateUser_Other_ReturnsNoContent()
    {
        SetCurrentUser(1);

        var result = await _sut.DeactivateUser(2);

        result.Should().BeOfType<NoContentResult>();
        _authService.Verify(s => s.DeactivateUserAsync(2, It.IsAny<CancellationToken>()), Times.Once);
        _jwtService.Verify(s => s.RevokeAllTokensForUserAsync(2, It.IsAny<CancellationToken>()), Times.Once);
    }
}
