import Foundation
import CoreMedia

/// Protocol defining the interface for audio capture backends.
/// Enables testability through protocol abstraction and allows future alternative implementations.
protocol AudioCaptureProvider {
    /// Starts audio capture from configured sources.
    /// - Throws: Errors related to permissions, device availability, or capture initialization.
    func startCapture() async throws
    
    /// Stops audio capture and releases resources.
    func stopCapture() async
    
    /// Stream of audio data events from microphone and system audio sources.
    var audioDataStream: AsyncStream<AudioData> { get }
}
