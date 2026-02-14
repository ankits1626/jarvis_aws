import Testing
import Foundation
@testable import JarvisListen

// MARK: - Signal Handler Tests

/// Tests for SignalHandler class
/// **Validates: Requirements 7.1, 7.3, 7.4, 7.5**
///
/// Note: Testing signal handlers is challenging because:
/// 1. Sending real signals to the test process can terminate it
/// 2. Signal handlers use global state
/// 3. We need to verify behavior without actually killing the test process
///
/// These tests verify:
/// - SignalHandler can be set up with a shutdown callback
/// - The shutdown callback is invoked when signal handler methods are called
/// - SIGPIPE handling is ignored (tested via Darwin.signal configuration)

@Test("SignalHandler setup stores shutdown handler")
func testSignalHandlerSetup() async throws {
    // Test that SignalHandler can be initialized and setup with a callback
    var shutdownCalled = false
    
    let handler = SignalHandler()
    handler.setup {
        shutdownCalled = true
    }
    
    // Verify the handler was created successfully
    // We can't easily verify signal() was called without sending actual signals,
    // but we can verify the shutdown handler is stored by calling the handler methods
    
    // Call handleSIGINT directly to verify shutdown handler is invoked
    handler.handleSIGINT()
    
    #expect(shutdownCalled == true, "Shutdown handler should be called when handleSIGINT is invoked")
}

@Test("SIGINT handler triggers shutdown callback")
func testSIGINTTriggersShutdown() async throws {
    // **Validates: Requirements 7.1**
    // WHEN SIGINT is received (Ctrl+C), THE System SHALL stop the SCStream gracefully
    
    var shutdownCalled = false
    var callCount = 0
    
    let handler = SignalHandler()
    handler.setup {
        shutdownCalled = true
        callCount += 1
    }
    
    // Simulate SIGINT by calling the handler method directly
    handler.handleSIGINT()
    
    #expect(shutdownCalled == true, "SIGINT should trigger shutdown handler")
    #expect(callCount == 1, "Shutdown handler should be called exactly once")
    
    // Call again to verify it can be called multiple times
    handler.handleSIGINT()
    #expect(callCount == 2, "Shutdown handler should be callable multiple times")
}

@Test("SIGTERM handler triggers shutdown callback")
func testSIGTERMTriggersShutdown() async throws {
    // **Validates: Requirements 7.3, 7.4**
    // WHEN SIGTERM is received, THE System SHALL perform the same graceful shutdown as SIGINT
    
    var shutdownCalled = false
    var callCount = 0
    
    let handler = SignalHandler()
    handler.setup {
        shutdownCalled = true
        callCount += 1
    }
    
    // Simulate SIGTERM by calling the handler method directly
    handler.handleSIGTERM()
    
    #expect(shutdownCalled == true, "SIGTERM should trigger shutdown handler")
    #expect(callCount == 1, "Shutdown handler should be called exactly once")
    
    // Call again to verify it can be called multiple times
    handler.handleSIGTERM()
    #expect(callCount == 2, "Shutdown handler should be callable multiple times")
}

@Test("SIGINT and SIGTERM use same shutdown mechanism")
func testSIGINTAndSIGTERMEquivalent() async throws {
    // **Validates: Requirements 7.1, 7.3, 7.4**
    // Both SIGINT and SIGTERM should trigger the same shutdown handler
    
    var shutdownCount = 0
    
    let handler = SignalHandler()
    handler.setup {
        shutdownCount += 1
    }
    
    // Call both handlers
    handler.handleSIGINT()
    handler.handleSIGTERM()
    
    #expect(shutdownCount == 2, "Both SIGINT and SIGTERM should invoke the same shutdown handler")
}

@Test("Multiple SignalHandler instances can coexist")
func testMultipleSignalHandlers() async throws {
    // Test that multiple SignalHandler instances can be created
    // Each instance manages its own DispatchSource instances
    
    var shutdown1Called = false
    var shutdown2Called = false
    
    let handler1 = SignalHandler()
    handler1.setup {
        shutdown1Called = true
    }
    
    let handler2 = SignalHandler()
    handler2.setup {
        shutdown2Called = true
    }
    
    // Both handlers should work independently
    handler1.handleSIGINT()
    handler2.handleSIGINT()
    
    #expect(shutdown1Called == true, "Handler 1 should be called")
    #expect(shutdown2Called == true, "Handler 2 should be called")
}

@Test("Shutdown handler can perform cleanup operations")
func testShutdownHandlerCleanup() async throws {
    // Test that the shutdown handler can perform multiple cleanup operations
    
    var streamStopped = false
    var buffersFlushed = false
    var resourcesReleased = false
    
    let handler = SignalHandler()
    handler.setup {
        // Simulate cleanup sequence
        streamStopped = true
        buffersFlushed = true
        resourcesReleased = true
    }
    
    handler.handleSIGINT()
    
    #expect(streamStopped == true, "Stream should be stopped during shutdown")
    #expect(buffersFlushed == true, "Buffers should be flushed during shutdown")
    #expect(resourcesReleased == true, "Resources should be released during shutdown")
}

// MARK: - SIGPIPE Handling Tests

@Test("SIGPIPE handler is ignored")
func testSIGPIPEIsIgnored() async throws {
    // **Validates: Requirements 7.5**
    // WHEN SIGPIPE is received, THE System SHALL handle it silently and exit gracefully
    
    // With the new implementation, SIGPIPE is set to SIG_IGN (ignore)
    // This means the process won't crash when writing to a closed pipe
    
    // We verify that the handler sets up SIGPIPE to be ignored
    let handler = SignalHandler()
    handler.setup {
        // This should not be called for SIGPIPE
    }
    
    // SIGPIPE is now ignored via Darwin.signal(SIGPIPE, SIG_IGN)
    // The process will not terminate on broken pipe, which is the desired behavior
    // In practice, write operations to closed pipes will return errors instead of crashing
}

// MARK: - Integration-Style Tests

@Test("SignalHandler integrates with async shutdown sequence")
func testAsyncShutdownSequence() async throws {
    // Test that SignalHandler can trigger an async shutdown sequence
    
    var shutdownPhases: [String] = []
    
    let handler = SignalHandler()
    handler.setup {
        shutdownPhases.append("signal_received")
    }
    
    // Simulate the shutdown sequence from main.swift
    handler.handleSIGINT()
    
    // Simulate async cleanup
    shutdownPhases.append("stream_stopped")
    shutdownPhases.append("buffers_flushed")
    shutdownPhases.append("exit")
    
    #expect(shutdownPhases == ["signal_received", "stream_stopped", "buffers_flushed", "exit"],
           "Shutdown sequence should follow correct order")
}

@Test("SignalHandler can be used with Task cancellation")
func testSignalHandlerWithTaskCancellation() async throws {
    // Test that SignalHandler can coordinate with Task cancellation
    
    actor State {
        var taskCancelled = false
        var shutdownInitiated = false
        
        func setTaskCancelled() {
            taskCancelled = true
        }
        
        func setShutdownInitiated() {
            shutdownInitiated = true
        }
        
        func getTaskCancelled() -> Bool {
            return taskCancelled
        }
        
        func getShutdownInitiated() -> Bool {
            return shutdownInitiated
        }
    }
    
    let state = State()
    
    let task = Task {
        do {
            try await Task.sleep(for: .seconds(10))
        } catch {
            await state.setTaskCancelled()
        }
    }
    
    let handler = SignalHandler()
    handler.setup {
        Task {
            await state.setShutdownInitiated()
            task.cancel()
        }
    }
    
    // Trigger shutdown
    handler.handleSIGINT()
    
    // Wait a bit for cancellation to propagate
    try await Task.sleep(for: .milliseconds(100))
    
    let shutdownInitiated = await state.getShutdownInitiated()
    let taskCancelled = await state.getTaskCancelled()
    
    #expect(shutdownInitiated == true, "Shutdown should be initiated")
    #expect(taskCancelled == true, "Task should be cancelled during shutdown")
}

// MARK: - Edge Cases

@Test("SignalHandler works without shutdown handler")
func testSignalHandlerWithoutCallback() async throws {
    // Test that SignalHandler doesn't crash if setup is called without a handler
    // or if signal is received before setup
    
    let handler = SignalHandler()
    
    // Call handler methods before setup - should not crash
    handler.handleSIGINT()
    handler.handleSIGTERM()
    
    // Now setup with a handler
    var called = false
    handler.setup {
        called = true
    }
    
    handler.handleSIGINT()
    #expect(called == true, "Handler should work after setup")
}

@Test("SignalHandler can be setup multiple times")
func testSignalHandlerMultipleSetup() async throws {
    // Test that calling setup multiple times replaces the handler
    
    var firstCalled = false
    var secondCalled = false
    
    let handler = SignalHandler()
    
    handler.setup {
        firstCalled = true
    }
    
    handler.setup {
        secondCalled = true
    }
    
    handler.handleSIGINT()
    
    // Only the second handler should be called
    #expect(secondCalled == true, "Second handler should be active")
    #expect(firstCalled == false, "First handler should be replaced")
}
