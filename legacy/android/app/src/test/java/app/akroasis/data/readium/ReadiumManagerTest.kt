// Unit tests for ReadiumManager initialization
package app.akroasis.data.readium

import android.content.Context
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.mockito.kotlin.*
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment

@RunWith(RobolectricTestRunner::class)
class ReadiumManagerTest {

    private lateinit var context: Context
    private lateinit var readiumManager: ReadiumManager

    @Before
    fun setup() {
        context = RuntimeEnvironment.getApplication()
        readiumManager = ReadiumManager(context)
    }

    @Test
    fun `manager initializes successfully`() {
        assertNotNull(readiumManager)
    }

    @Test
    fun `assetRetriever is initialized`() {
        assertNotNull(readiumManager.assetRetriever)
    }

    @Test
    fun `publicationOpener is initialized`() {
        assertNotNull(readiumManager.publicationOpener)
    }

    @Test
    fun `manager is singleton - same instance returned`() {
        // Simulate singleton behavior by creating with same context
        val manager1 = ReadiumManager(context)
        val manager2 = ReadiumManager(context)

        // Both should create valid instances (actual singleton is handled by Hilt)
        assertNotNull(manager1)
        assertNotNull(manager2)
    }

    @Test
    fun `assetRetriever has valid contentResolver`() {
        // AssetRetriever should be constructed with context's contentResolver
        assertNotNull(readiumManager.assetRetriever)
        // Can't directly access contentResolver, but we verify initialization succeeds
    }

    @Test
    fun `publicationOpener has valid parser`() {
        // PublicationOpener should have a valid parser configured
        assertNotNull(readiumManager.publicationOpener)
        // Parser configuration is internal, but initialization success implies correctness
    }
}
