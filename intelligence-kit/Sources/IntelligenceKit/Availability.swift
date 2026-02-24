import Foundation
import FoundationModels

// MARK: - AvailabilityChecker

/// Checks if Foundation Models (SystemLanguageModel) is available on this system.
///
/// Availability depends on three factors:
/// 1. **Hardware**: Must be Apple Silicon (arm64)
/// 2. **macOS Version**: Must be macOS 26.0 or later
/// 3. **Model Status**: Apple Intelligence must be enabled and the model must be ready
///
/// The check is synchronous and completes immediately (no timeout needed).
///
/// Example usage:
/// ```swift
/// let checker = AvailabilityChecker()
/// let (available, reason) = checker.check()
/// if available {
///     print("Foundation Models is available")
/// } else {
///     print("Unavailable: \(reason ?? "unknown")")
/// }
/// ```
struct AvailabilityChecker {
    
    /// Checks if Foundation Models is available on this system.
    ///
    /// - Returns: A tuple containing:
    ///   - `available`: `true` if Foundation Models can be used, `false` otherwise
    ///   - `reason`: A human-readable explanation when unavailable, `nil` when available
    ///
    /// Possible unavailability reasons:
    /// - "Apple Silicon required" - Running on Intel hardware
    /// - "macOS 26.0 or later required" - macOS version too old
    /// - "Apple Intelligence not enabled by user" - User hasn't enabled the feature
    /// - "Device not eligible for Apple Intelligence" - Hardware doesn't support it
    /// - "Language model not ready yet" - Model is still downloading or initializing
    ///
    /// The check is synchronous and returns immediately. The design specifies
    /// 2-second completion, but in practice this is a simple property access.
    func check() -> (available: Bool, reason: String?) {
        // Check hardware - Foundation Models requires Apple Silicon
        #if !arch(arm64)
        return (false, "Apple Silicon required")
        #endif
        
        // Check macOS version - Foundation Models requires macOS 26.0+
        let osVersion = ProcessInfo.processInfo.operatingSystemVersion
        if osVersion.majorVersion < 26 {
            return (false, "macOS 26.0 or later required")
        }
        
        // Check model availability
        // Note: SystemLanguageModel.default.availability is a synchronous property,
        // so no timeout is needed. The design specifies 2-second completion, but in
        // practice this is a simple property access that returns immediately.
        let availability = SystemLanguageModel.default.availability
        
        switch availability {
        case .available:
            return (true, nil)
        case .unavailable(let reason):
            let reasonText = switch reason {
            case .appleIntelligenceNotEnabled:
                "Apple Intelligence not enabled by user"
            case .deviceNotEligible:
                "Device not eligible for Apple Intelligence"
            case .modelNotReady:
                "Language model not ready yet"
            @unknown default:
                "Unknown unavailability reason"
            }
            return (false, reasonText)
        @unknown default:
            return (false, "Unknown availability status")
        }
    }
}
