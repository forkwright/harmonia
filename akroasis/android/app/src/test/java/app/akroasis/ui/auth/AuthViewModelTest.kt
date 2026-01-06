package app.akroasis.ui.auth

import androidx.arch.core.executor.testing.InstantTaskExecutorRule
import app.akroasis.data.api.AuthInterceptor
import app.akroasis.data.api.LoginRequest
import app.akroasis.data.api.LoginResponse
import app.akroasis.data.api.MouseionApi
import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.mockito.kotlin.*
import retrofit2.Response

@OptIn(ExperimentalCoroutinesApi::class)
class AuthViewModelTest {

    @get:Rule
    val instantTaskExecutorRule = InstantTaskExecutorRule()

    private lateinit var viewModel: AuthViewModel
    private lateinit var api: MouseionApi
    private lateinit var authInterceptor: AuthInterceptor

    private val testDispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)

        api = mock()
        authInterceptor = mock()
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun `init with existing refresh token sets Authenticated state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn("refresh-token")

        viewModel = AuthViewModel(api, authInterceptor)

        viewModel.authState.test {
            assertEquals(AuthState.Authenticated, awaitItem())
        }
    }

    @Test
    fun `init without refresh token sets Unauthenticated state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)

        viewModel = AuthViewModel(api, authInterceptor)

        viewModel.authState.test {
            assertEquals(AuthState.Unauthenticated, awaitItem())
        }
    }

    @Test
    fun `login with blank username sets Error state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        viewModel.login("", "password", "http://server.com")
        testDispatcher.scheduler.advanceUntilIdle()

        viewModel.authState.test {
            val state = awaitItem()
            assertTrue(state is AuthState.Error)
            assertEquals("Username, password, and server URL are required", (state as AuthState.Error).message)
        }
    }

    @Test
    fun `login with blank password sets Error state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        viewModel.login("username", "", "http://server.com")
        testDispatcher.scheduler.advanceUntilIdle()

        viewModel.authState.test {
            val state = awaitItem()
            assertTrue(state is AuthState.Error)
            assertEquals("Username, password, and server URL are required", (state as AuthState.Error).message)
        }
    }

    @Test
    fun `login with blank server URL sets Error state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        viewModel.login("username", "password", "")
        testDispatcher.scheduler.advanceUntilIdle()

        viewModel.authState.test {
            val state = awaitItem()
            assertTrue(state is AuthState.Error)
            assertEquals("Username, password, and server URL are required", (state as AuthState.Error).message)
        }
    }

    @Test
    fun `login success saves tokens and sets Authenticated state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        val loginResponse = LoginResponse(
            accessToken = "access-token",
            refreshToken = "refresh-token",
            expiresIn = 3600,
            userId = "user-123"
        )
        val response = Response.success(loginResponse)
        whenever(api.login(any())).thenReturn(response)

        viewModel.authState.test {
            // Initial state
            assertEquals(AuthState.Unauthenticated, awaitItem())

            viewModel.login("username", "password", "http://server.com")
            testDispatcher.scheduler.advanceUntilIdle()

            // Loading state
            assertEquals(AuthState.Loading, awaitItem())

            // Authenticated state
            assertEquals(AuthState.Authenticated, awaitItem())
        }

        verify(authInterceptor).saveTokens("access-token", "refresh-token")
        verify(authInterceptor).saveServerUrl("http://server.com")
    }

    @Test
    fun `login with 401 error sets error state with invalid credentials message`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        val response = Response.error<LoginResponse>(401, okhttp3.ResponseBody.create(null, ""))
        whenever(api.login(any())).thenReturn(response)

        viewModel.authState.test {
            assertEquals(AuthState.Unauthenticated, awaitItem())

            viewModel.login("username", "wrong-password", "http://server.com")
            testDispatcher.scheduler.advanceUntilIdle()

            assertEquals(AuthState.Loading, awaitItem())

            val errorState = awaitItem()
            assertTrue(errorState is AuthState.Error)
            assertEquals("Invalid username or password", (errorState as AuthState.Error).message)
        }

        verify(authInterceptor, never()).saveTokens(any(), any())
    }

    @Test
    fun `login with 404 error sets error state with server not found message`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        val response = Response.error<LoginResponse>(404, okhttp3.ResponseBody.create(null, ""))
        whenever(api.login(any())).thenReturn(response)

        viewModel.authState.test {
            assertEquals(AuthState.Unauthenticated, awaitItem())

            viewModel.login("username", "password", "http://wrong-server.com")
            testDispatcher.scheduler.advanceUntilIdle()

            assertEquals(AuthState.Loading, awaitItem())

            val errorState = awaitItem()
            assertTrue(errorState is AuthState.Error)
            assertEquals("Server not found. Check the URL.", (errorState as AuthState.Error).message)
        }
    }

    @Test
    fun `login with network exception sets error state with connection failed message`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        whenever(api.login(any())).thenThrow(RuntimeException("Network error"))

        viewModel.authState.test {
            assertEquals(AuthState.Unauthenticated, awaitItem())

            viewModel.login("username", "password", "http://server.com")
            testDispatcher.scheduler.advanceUntilIdle()

            assertEquals(AuthState.Loading, awaitItem())

            val errorState = awaitItem()
            assertTrue(errorState is AuthState.Error)
            assertEquals("Connection failed: Network error", (errorState as AuthState.Error).message)
        }
    }

    @Test
    fun `login with empty response body sets error state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn(null)
        viewModel = AuthViewModel(api, authInterceptor)

        val response = Response.success<LoginResponse>(null)
        whenever(api.login(any())).thenReturn(response)

        viewModel.authState.test {
            assertEquals(AuthState.Unauthenticated, awaitItem())

            viewModel.login("username", "password", "http://server.com")
            testDispatcher.scheduler.advanceUntilIdle()

            assertEquals(AuthState.Loading, awaitItem())

            val errorState = awaitItem()
            assertTrue(errorState is AuthState.Error)
            assertEquals("Login failed: empty response", (errorState as AuthState.Error).message)
        }
    }

    @Test
    fun `logout clears tokens and sets Unauthenticated state`() = runTest {
        whenever(authInterceptor.getRefreshToken()).thenReturn("refresh-token")
        viewModel = AuthViewModel(api, authInterceptor)

        viewModel.authState.test {
            assertEquals(AuthState.Authenticated, awaitItem())

            viewModel.logout()

            assertEquals(AuthState.Unauthenticated, awaitItem())
        }

        verify(authInterceptor).clearTokens()
    }
}
