# Test Coverage - Push to 80%

## Current Status

**Goal**: 80% code coverage
**Previous Status**: 42-52% (375+ tests)
**Current Status**: Running baseline assessment

## Java Version Requirement

Tests require Java 21 due to AGP 8.7.3 compatibility:
```bash
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
./gradlew :app:testDebugUnitTestCoverage
```

## Recent Fixes (2026-01-07)

### Test Compilation Issues

1. **PlayerViewModel Tests** - Added missing `mediaRepository` parameter
   - Phase1FeaturesTest.kt
   - PlayerViewModelTest.kt

2. **EbookViewModel Tests** - Added missing `readiumManager` parameter
   - EbookViewModelTest.kt
   - Fixed float assertion delta parameters
   - Simplified loading state test (removed backgroundScope.launch)

3. **ContinueFeedScreen** - Removed onLongPress parameter call (parameter removed in PR #124)

### Jacoco Configuration

- Added jacoco plugin to app/build.gradle.kts
- Enabled test coverage for debug builds
- Created testDebugUnitTestCoverage task
- Configured file filters (excludes generated code, Hilt, R classes)
- HTML and XML reports enabled

## Coverage Report Location

After running `testDebugUnitTestCoverage`:
```
app/build/reports/jacoco/testDebugUnitTestCoverage/html/index.html
```

## Test Structure

### Existing Tests (21 files)

**Audio Engine** (6 files):
- CrossfeedEngineTest.kt
- EqualizerEngineTest.kt
- GaplessPlaybackEngineTest.kt
- HeadroomManagerTest.kt
- PlaybackQueueTest.kt
- TrackLoaderTest.kt

**Data Layer** (5 files):
- PlaybackStateStoreTest.kt
- PlaybackSpeedPreferencesTest.kt
- AutoEQRepositoryTest.kt
- AudiobookRepositoryTest.kt
- EbookRepositoryTest.kt
- MediaRepositoryTest.kt

**Integration** (2 files):
- AudioPipelineIntegrationTest.kt
- PlaybackQueueIntegrationTest.kt

**UI Layer** (5 files):
- PlayerViewModelTest.kt
- Phase1FeaturesTest.kt
- QueueExporterTest.kt
- AuthViewModelTest.kt
- EbookViewModelTest.kt

**Scrobbling** (1 file):
- ScrobbleManagerTest.kt

**Utils** (1 file):
- MainDispatcherRule.kt

### Missing Test Coverage

**Critical Gaps (needed for 80%)**:

1. **EPUB Reader (Phase 5 Week 3)**
   - ReadiumManager.kt - No tests
   - EbookReaderScreen.kt - UI tests needed
   - Readium integration tests

2. **Continue Feed (Phase 5 Week 4)**
   - ContinueFeedViewModel.kt - No tests
   - ContinueFeedScreen.kt - UI tests needed
   - ContinueCard.kt - UI tests needed

3. **Multi-Media Support (Phase 4)**
   - MediaRepository unified progress tracking
   - Session management
   - Multi-type search

4. **DSP Engine (Phase 3)**
   - Additional EQ profile tests
   - A/B comparison tests
   - DSP bypass tests

5. **End-to-End Flows**
   - Play music → background → resume
   - Read ebook → close → reopen at position
   - Navigate between media types
   - Offline queue management

6. **Edge Cases**
   - Network failure recovery
   - Corrupted file handling
   - Large library performance
   - USB DAC hot-plug

## Test Writing Plan

### Priority 1: Critical New Features (15-20% coverage gain)

1. ReadiumManager integration tests
2. ContinueFeedViewModel unit tests
3. Continue feed UI tests

### Priority 2: Repository Layer (10-15% coverage gain)

1. MediaRepository complete coverage
2. EbookRepository edge cases
3. AudiobookRepository edge cases

### Priority 3: End-to-End Flows (5-10% coverage gain)

1. Playback lifecycle tests
2. Reading lifecycle tests
3. Cross-media navigation tests

### Priority 4: Edge Cases (5-10% coverage gain)

1. Error handling paths
2. Network failure scenarios
3. File corruption handling
4. Resource cleanup tests

## Running Tests

### All Tests
```bash
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
./gradlew :app:testDebugUnitTest
```

### With Coverage
```bash
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
./gradlew :app:testDebugUnitTestCoverage
```

### Specific Test
```bash
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
./gradlew :app:testDebugUnitTest --tests "app.akroasis.ui.ebook.EbookViewModelTest"
```

## Next Steps

1. ✅ Fix test compilation issues
2. ✅ Configure Jacoco
3. ⏳ Get baseline coverage numbers
4. Add ReadiumManager tests
5. Add ContinueFeedViewModel tests
6. Add Continue feed UI tests
7. Add end-to-end flow tests
8. Add edge case tests
9. Verify 80% target achieved
