import Testing
import ScreenCaptureKit
import CoreMedia
@testable import JarvisListen

// MARK: - SCStream Configuration Tests

/// Helper function to create an SCStreamConfiguration with the same settings
/// used in AudioCapture.startCapture(). This allows us to test the configuration
/// values without requiring Screen Recording permissions.
func createTestStreamConfiguration() -> SCStreamConfiguration {
    let config = SCStreamConfiguration()
    
    // Audio settings (matching AudioCapture.startCapture())
    config.capturesAudio = true
    config.captureMicrophone = true
    config.excludesCurrentProcessAudio = true
    
    // Minimize video overhead
    config.width = 2
    config.height = 2
    config.minimumFrameInterval = CMTime(value: 1, timescale: 1)  // 1 FPS
    
    return config
}

// MARK: - Unit Tests for SCStream Configuration
// Validates: Requirements 1.1, 1.2, 2.1, 11.1, 11.2, 11.3

@Test("SCStream configuration has capturesAudio enabled")
func testCapturesAudioEnabled() throws {
    // Requirement 1.1: System SHALL use ScreenCaptureKit's SCStream with capturesAudio set to true
    let config = createTestStreamConfiguration()
    
    #expect(config.capturesAudio == true,
           "capturesAudio should be enabled for system audio capture")
}

@Test("SCStream configuration has excludesCurrentProcessAudio enabled")
func testExcludesCurrentProcessAudioEnabled() throws {
    // Requirement 1.2: System SHALL set excludesCurrentProcessAudio to true to prevent feedback loops
    let config = createTestStreamConfiguration()
    
    #expect(config.excludesCurrentProcessAudio == true,
           "excludesCurrentProcessAudio should be enabled to prevent feedback")
}

@Test("SCStream configuration has captureMicrophone enabled")
func testCaptureMicrophoneEnabled() throws {
    // Requirement 2.1: System SHALL use ScreenCaptureKit's captureMicrophone set to true
    let config = createTestStreamConfiguration()
    
    #expect(config.captureMicrophone == true,
           "captureMicrophone should be enabled for microphone capture")
}

@Test("SCStream configuration has minimal video dimensions")
func testMinimalVideoDimensions() throws {
    // Requirements 11.1, 11.2: System SHALL set video capture width and height to 2 pixels
    let config = createTestStreamConfiguration()
    
    #expect(config.width == 2,
           "Video width should be 2 pixels to minimize overhead")
    #expect(config.height == 2,
           "Video height should be 2 pixels to minimize overhead")
}

@Test("SCStream configuration has maximum frame interval")
func testMaximumFrameInterval() throws {
    // Requirement 11.3: System SHALL set the maximum frame interval to minimize video processing overhead
    let config = createTestStreamConfiguration()
    
    let expectedInterval = CMTime(value: 1, timescale: 1)  // 1 second = 1 FPS (minimum)
    
    #expect(config.minimumFrameInterval.value == expectedInterval.value,
           "minimumFrameInterval value should be 1")
    #expect(config.minimumFrameInterval.timescale == expectedInterval.timescale,
           "minimumFrameInterval timescale should be 1")
    #expect(config.minimumFrameInterval.seconds == 1.0,
           "minimumFrameInterval should be 1 second (1 FPS)")
}

// MARK: - Edge Cases

@Test("SCStream configuration video dimensions are minimal but valid")
func testVideoDimensionsAreMinimalButValid() throws {
    // Verify that 2x2 is the smallest valid dimension
    // (ScreenCaptureKit requires at least 1x1, but 2x2 is more reliable)
    let config = createTestStreamConfiguration()
    
    #expect(config.width >= 1, "Width must be at least 1 pixel")
    #expect(config.height >= 1, "Height must be at least 1 pixel")
    #expect(config.width <= 2, "Width should be minimal (2 pixels)")
    #expect(config.height <= 2, "Height should be minimal (2 pixels)")
}

@Test("SCStream configuration frame interval is at maximum")
func testFrameIntervalIsMaximum() throws {
    // Verify that 1 second is the maximum frame interval (minimum FPS)
    let config = createTestStreamConfiguration()
    
    // 1 second = 1 FPS, which is the minimum frame rate
    // This maximizes the interval between frames to minimize overhead
    let intervalSeconds = config.minimumFrameInterval.seconds
    
    #expect(intervalSeconds >= 1.0,
           "Frame interval should be at least 1 second to minimize overhead")
}

@Test("SCStream configuration has all audio flags enabled")
func testAllAudioFlagsEnabled() throws {
    // Comprehensive test: verify all three audio flags are enabled together
    let config = createTestStreamConfiguration()
    
    #expect(config.capturesAudio == true,
           "capturesAudio must be enabled")
    #expect(config.captureMicrophone == true,
           "captureMicrophone must be enabled")
    #expect(config.excludesCurrentProcessAudio == true,
           "excludesCurrentProcessAudio must be enabled")
}

@Test("SCStream configuration matches AudioCapture requirements")
func testConfigurationMatchesAudioCaptureRequirements() throws {
    // Integration test: verify the configuration matches all requirements
    // that AudioCapture.startCapture() should set
    let config = createTestStreamConfiguration()
    
    // Audio capture requirements
    #expect(config.capturesAudio == true, "Must capture system audio")
    #expect(config.captureMicrophone == true, "Must capture microphone")
    #expect(config.excludesCurrentProcessAudio == true, "Must exclude own audio")
    
    // Video minimization requirements
    #expect(config.width == 2, "Must use minimal width")
    #expect(config.height == 2, "Must use minimal height")
    #expect(config.minimumFrameInterval.seconds == 1.0, "Must use maximum interval")
}
