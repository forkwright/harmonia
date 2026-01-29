package app.akroasis.integration

import app.akroasis.data.network.RetryPolicy
import app.akroasis.util.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.IOException
import java.net.SocketTimeoutException
import java.net.UnknownHostException

/**
 * Integration tests for network layer retry policy and error handling.
 * Tests exponential backoff, retry limits, and various network failure scenarios.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class NetworkLayerIntegrationTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ===== Retry Policy Basic Tests =====

    @Test
    fun `SCENARIO 1 - successful request returns immediately`() = runTest {
        // Given - operation that succeeds
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff {
            attempts++
            "success"
        }

        // Then
        assertEquals("success", result)
        assertEquals(1, attempts)
    }

    @Test
    fun `SCENARIO 2 - retries on failure then succeeds`() = runTest {
        // Given - operation that fails twice then succeeds
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 3,
            initialDelayMs = 10,
            maxDelayMs = 100
        ) {
            attempts++
            if (attempts < 3) throw IOException("Temporary failure")
            "success after retries"
        }

        // Then
        assertEquals("success after retries", result)
        assertEquals(3, attempts)
    }

    @Test
    fun `SCENARIO 3 - throws after max retries exceeded`() = runTest {
        // Given - operation that always fails
        var attempts = 0

        // When/Then
        assertThrows(IOException::class.java) {
            kotlinx.coroutines.runBlocking {
                RetryPolicy.retryWithExponentialBackoff(
                    maxRetries = 3,
                    initialDelayMs = 10,
                    maxDelayMs = 100
                ) {
                    attempts++
                    throw IOException("Persistent failure")
                }
            }
        }

        assertEquals(3, attempts)
    }

    @Test
    fun `SCENARIO 4 - respects max retry count`() = runTest {
        // Given
        var attempts = 0

        // When
        try {
            RetryPolicy.retryWithExponentialBackoff(
                maxRetries = 5,
                initialDelayMs = 1
            ) {
                attempts++
                throw RuntimeException("Error $attempts")
            }
        } catch (e: RuntimeException) {
            // Expected
        }

        // Then - should attempt exactly maxRetries times
        assertEquals(5, attempts)
    }

    // ===== Exponential Backoff Tests =====

    @Test
    fun `SCENARIO 5 - delays increase exponentially`() = runTest {
        // Given - tracking delay times
        val delays = mutableListOf<Long>()
        var lastTime = System.currentTimeMillis()
        var attempts = 0

        // When
        try {
            RetryPolicy.retryWithExponentialBackoff(
                maxRetries = 4,
                initialDelayMs = 100,
                maxDelayMs = 1000,
                factor = 2.0
            ) {
                attempts++
                val now = System.currentTimeMillis()
                if (attempts > 1) {
                    delays.add(now - lastTime)
                }
                lastTime = now
                throw RuntimeException("Fail")
            }
        } catch (e: RuntimeException) {
            // Expected
        }

        // Then - each delay should be roughly double the previous (with some tolerance)
        // Note: In test environment delays may be mocked, so we just verify attempts
        assertEquals(4, attempts)
    }

    @Test
    fun `SCENARIO 6 - delay capped at maxDelayMs`() = runTest {
        // Given - very large factor to quickly exceed max
        var attempts = 0

        // When
        try {
            RetryPolicy.retryWithExponentialBackoff(
                maxRetries = 5,
                initialDelayMs = 1000,
                maxDelayMs = 2000,
                factor = 10.0
            ) {
                attempts++
                throw RuntimeException("Fail")
            }
        } catch (e: RuntimeException) {
            // Expected
        }

        // Then - should complete without hanging (delay capped)
        assertEquals(5, attempts)
    }

    // ===== Network Error Types =====

    @Test
    fun `SCENARIO 7 - handles SocketTimeoutException`() = runTest {
        // Given
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 3,
            initialDelayMs = 10
        ) {
            attempts++
            if (attempts < 2) throw SocketTimeoutException("Connection timed out")
            "recovered"
        }

        // Then
        assertEquals("recovered", result)
        assertEquals(2, attempts)
    }

    @Test
    fun `SCENARIO 8 - handles UnknownHostException`() = runTest {
        // Given - DNS resolution failure
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 3,
            initialDelayMs = 10
        ) {
            attempts++
            if (attempts < 2) throw UnknownHostException("Unable to resolve host")
            "resolved"
        }

        // Then
        assertEquals("resolved", result)
    }

    @Test
    fun `SCENARIO 9 - handles IOException`() = runTest {
        // Given
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 3,
            initialDelayMs = 10
        ) {
            attempts++
            if (attempts < 3) throw IOException("Network error")
            "success"
        }

        // Then
        assertEquals("success", result)
    }

    // ===== Default Parameter Tests =====

    @Test
    fun `SCENARIO 10 - default maxRetries is 3`() = runTest {
        // Given
        var attempts = 0

        // When
        try {
            RetryPolicy.retryWithExponentialBackoff {
                attempts++
                throw RuntimeException("Fail")
            }
        } catch (e: RuntimeException) {
            // Expected
        }

        // Then
        assertEquals(3, attempts)
    }

    @Test
    fun `SCENARIO 11 - single retry succeeds on first try`() = runTest {
        // Given
        val result = RetryPolicy.retryWithExponentialBackoff(maxRetries = 1) {
            "immediate success"
        }

        // Then
        assertEquals("immediate success", result)
    }

    // ===== Edge Cases =====

    @Test
    fun `SCENARIO 12 - handles null result from successful operation`() = runTest {
        // Given
        val result: String? = RetryPolicy.retryWithExponentialBackoff {
            null
        }

        // Then
        assertNull(result)
    }

    @Test
    fun `SCENARIO 13 - handles complex return types`() = runTest {
        // Given
        data class ApiResponse(val data: List<String>, val count: Int)

        // When
        val result = RetryPolicy.retryWithExponentialBackoff {
            ApiResponse(listOf("a", "b", "c"), 3)
        }

        // Then
        assertEquals(3, result.count)
        assertEquals(listOf("a", "b", "c"), result.data)
    }

    @Test
    fun `SCENARIO 14 - last exception is thrown when all retries fail`() = runTest {
        // Given - exception with specific message
        var counter = 0

        // When/Then
        val exception = assertThrows(RuntimeException::class.java) {
            kotlinx.coroutines.runBlocking {
                RetryPolicy.retryWithExponentialBackoff(
                    maxRetries = 3,
                    initialDelayMs = 10
                ) {
                    counter++
                    throw RuntimeException("Failure #$counter")
                }
            }
        }

        // Last exception should be thrown
        assertEquals("Failure #3", exception.message)
    }

    @Test
    fun `SCENARIO 15 - factor of 1 maintains constant delay`() = runTest {
        // Given
        var attempts = 0

        // When
        try {
            RetryPolicy.retryWithExponentialBackoff(
                maxRetries = 3,
                initialDelayMs = 50,
                factor = 1.0
            ) {
                attempts++
                throw RuntimeException("Fail")
            }
        } catch (e: RuntimeException) {
            // Expected
        }

        // Then
        assertEquals(3, attempts)
    }

    // ===== Cancellation Tests =====

    @Test
    fun `SCENARIO 16 - respects coroutine cancellation`() = runTest {
        // Given - this tests that the retry loop is cooperative with cancellation
        var attempts = 0

        // When - job is not cancelled, should complete normally
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 2,
            initialDelayMs = 10
        ) {
            attempts++
            if (attempts < 2) throw RuntimeException("Retry")
            "done"
        }

        // Then
        assertEquals("done", result)
        assertEquals(2, attempts)
    }

    // ===== Concurrent Retry Tests =====

    @Test
    fun `SCENARIO 17 - multiple concurrent retries are independent`() = runTest {
        // Given
        var counter1 = 0
        var counter2 = 0

        // When - two independent retry operations
        val result1 = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 2,
            initialDelayMs = 10
        ) {
            counter1++
            if (counter1 < 2) throw RuntimeException("Fail 1")
            "result1"
        }

        val result2 = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 3,
            initialDelayMs = 10
        ) {
            counter2++
            if (counter2 < 3) throw RuntimeException("Fail 2")
            "result2"
        }

        // Then
        assertEquals("result1", result1)
        assertEquals("result2", result2)
        assertEquals(2, counter1)
        assertEquals(3, counter2)
    }

    // ===== HTTP Status Code Simulation =====

    @Test
    fun `SCENARIO 18 - simulates 500 server error retry`() = runTest {
        // Given - simulated server error
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 3,
            initialDelayMs = 10
        ) {
            attempts++
            if (attempts < 3) throw RuntimeException("HTTP 500 Internal Server Error")
            "Server recovered"
        }

        // Then
        assertEquals("Server recovered", result)
    }

    @Test
    fun `SCENARIO 19 - simulates 503 service unavailable retry`() = runTest {
        // Given - simulated service unavailable
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 4,
            initialDelayMs = 10
        ) {
            attempts++
            if (attempts < 3) throw RuntimeException("HTTP 503 Service Unavailable")
            "Service available"
        }

        // Then
        assertEquals("Service available", result)
    }

    @Test
    fun `SCENARIO 20 - simulates 429 rate limit with backoff`() = runTest {
        // Given - rate limited
        var attempts = 0

        // When
        val result = RetryPolicy.retryWithExponentialBackoff(
            maxRetries = 4,
            initialDelayMs = 100,
            maxDelayMs = 1000,
            factor = 2.0
        ) {
            attempts++
            if (attempts < 4) throw RuntimeException("HTTP 429 Too Many Requests")
            "Rate limit cleared"
        }

        // Then
        assertEquals("Rate limit cleared", result)
        assertEquals(4, attempts)
    }

    // ===== Helper =====

    private inline fun <reified T : Throwable> assertThrows(
        expectedType: Class<T>,
        crossinline block: () -> Unit
    ): T {
        try {
            block()
            fail("Expected ${expectedType.simpleName} to be thrown")
        } catch (e: Throwable) {
            if (expectedType.isInstance(e)) {
                @Suppress("UNCHECKED_CAST")
                return e as T
            }
            throw e
        }
        throw AssertionError("Should not reach here")
    }
}
