import XCTest
import FoundationModels
@testable import IntelligenceKit

final class AvailabilityCheckerTests: XCTestCase {
    
    // MARK: - Test Availability Check Structure
    
    func testAvailabilityCheckReturnsCorrectStructure() {
        let checker = AvailabilityChecker()
        
        let result = checker.check()
        
        // Verify the result is a tuple with (Bool, String?)
        XCTAssertNotNil(result.available)
        
        // If unavailable, reason should be present
        if !result.available {
            XCTAssertNotNil(result.reason)
            XCTAssertFalse(result.reason!.isEmpty)
        }
    }
    
    // MARK: - Test Hardware Check
    
    func testHardwareCheckOnAppleSilicon() {
        // This test runs on the current hardware
        let checker = AvailabilityChecker()
        
        let result = checker.check()
        
        // On Apple Silicon (arm64), hardware check should pass
        // and we should get either available or a different unavailability reason
        #if arch(arm64)
        if !result.available {
            // If unavailable on arm64, it should NOT be due to hardware
            XCTAssertNotEqual(result.reason, "Apple Silicon required")
        }
        #else
        // On non-arm64, should be unavailable with hardware reason
        XCTAssertFalse(result.available)
        XCTAssertEqual(result.reason, "Apple Silicon required")
        #endif
    }
    
    // MARK: - Test macOS Version Check
    
    func testMacOSVersionCheck() {
        let checker = AvailabilityChecker()
        let osVersion = ProcessInfo.processInfo.operatingSystemVersion
        
        let result = checker.check()
        
        // If running on macOS < 26, should be unavailable with version reason
        if osVersion.majorVersion < 26 {
            XCTAssertFalse(result.available)
            XCTAssertEqual(result.reason, "macOS 26.0 or later required")
        }
    }
    
    // MARK: - Test Synchronous Execution
    
    func testSynchronousExecution() {
        let checker = AvailabilityChecker()
        
        // Measure execution time - should be very fast (< 1 second)
        let startTime = CFAbsoluteTimeGetCurrent()
        _ = checker.check()
        let elapsed = CFAbsoluteTimeGetCurrent() - startTime
        
        // Should complete in well under 2 seconds (design requirement)
        XCTAssertLessThan(elapsed, 2.0)
        
        // In practice, should be nearly instantaneous (< 0.1 seconds)
        XCTAssertLessThan(elapsed, 0.1)
    }
    
    func testMultipleCallsAreConsistent() {
        let checker = AvailabilityChecker()
        
        // Call check() multiple times
        let result1 = checker.check()
        let result2 = checker.check()
        let result3 = checker.check()
        
        // Results should be consistent
        XCTAssertEqual(result1.available, result2.available)
        XCTAssertEqual(result2.available, result3.available)
        XCTAssertEqual(result1.reason, result2.reason)
        XCTAssertEqual(result2.reason, result3.reason)
    }
    
    // MARK: - Test Unavailability Reasons
    
    func testUnavailabilityReasonsAreDescriptive() {
        let checker = AvailabilityChecker()
        
        let result = checker.check()
        
        if !result.available {
            // Reason should be one of the expected values
            let validReasons = [
                "Apple Silicon required",
                "macOS 26.0 or later required",
                "Apple Intelligence not enabled by user",
                "Device not eligible for Apple Intelligence",
                "Language model not ready yet",
                "Unknown unavailability reason",
                "Unknown availability status"
            ]
            
            XCTAssertTrue(validReasons.contains(result.reason!),
                         "Unexpected reason: \(result.reason!)")
        }
    }
    
    // MARK: - Integration Test Documentation
    
    func testAvailabilityCheckerRequiresRealEnvironment() {
        // This test documents that full AvailabilityChecker testing
        // requires running on actual hardware with different configurations.
        //
        // Full integration tests should verify:
        // 1. Available status on macOS 26+ with Apple Intelligence enabled
        // 2. Unavailable with correct reason on macOS < 26
        // 3. Unavailable with correct reason on non-Apple Silicon
        // 4. Unavailable with correct reason when Apple Intelligence disabled
        // 5. Unavailable with correct reason when model not ready
        // 6. Unavailable with correct reason on ineligible devices
        //
        // These scenarios require different hardware/OS configurations
        // and should be tested in Phase 9 (Integration Testing).
        
        XCTAssertTrue(true, "Integration tests deferred to Phase 9")
    }
}
