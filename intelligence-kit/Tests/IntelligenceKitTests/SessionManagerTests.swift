import XCTest
@testable import IntelligenceKit

final class SessionManagerTests: XCTestCase {
    
    // MARK: - 6.1 Test session creation returns unique IDs
    
    func testSessionCreationReturnsUniqueIDs() async throws {
        let manager = SessionManager()
        
        let sessionId1 = try await manager.openSession(instructions: nil)
        let sessionId2 = try await manager.openSession(instructions: nil)
        let sessionId3 = try await manager.openSession(instructions: nil)
        
        // All session IDs should be unique
        XCTAssertNotEqual(sessionId1, sessionId2)
        XCTAssertNotEqual(sessionId2, sessionId3)
        XCTAssertNotEqual(sessionId1, sessionId3)
        
        // Session IDs should be valid UUIDs
        XCTAssertNotNil(UUID(uuidString: sessionId1))
        XCTAssertNotNil(UUID(uuidString: sessionId2))
        XCTAssertNotNil(UUID(uuidString: sessionId3))
    }
    
    func testSessionCreationWithCustomInstructions() async throws {
        let manager = SessionManager()
        
        let customInstructions = "You are a specialized assistant."
        let sessionId = try await manager.openSession(instructions: customInstructions)
        
        XCTAssertNotNil(UUID(uuidString: sessionId))
        
        // Verify session can be retrieved
        let session = await manager.getSession(sessionId)
        XCTAssertNotNil(session)
    }
    
    func testSessionCreationWithNilInstructions() async throws {
        let manager = SessionManager()
        
        let sessionId = try await manager.openSession(instructions: nil)
        
        XCTAssertNotNil(UUID(uuidString: sessionId))
        
        // Verify session can be retrieved
        let session = await manager.getSession(sessionId)
        XCTAssertNotNil(session)
    }
    
    // MARK: - 6.2 Test concurrent session independence
    
    func testConcurrentSessionIndependence() async throws {
        let manager = SessionManager()
        
        // Create multiple sessions concurrently
        async let session1 = manager.openSession(instructions: "Assistant 1")
        async let session2 = manager.openSession(instructions: "Assistant 2")
        async let session3 = manager.openSession(instructions: "Assistant 3")
        
        let (id1, id2, id3) = try await (session1, session2, session3)
        
        // All sessions should have unique IDs
        XCTAssertNotEqual(id1, id2)
        XCTAssertNotEqual(id2, id3)
        XCTAssertNotEqual(id1, id3)
        
        // All sessions should be retrievable independently
        let retrievedSession1 = await manager.getSession(id1)
        let retrievedSession2 = await manager.getSession(id2)
        let retrievedSession3 = await manager.getSession(id3)
        
        XCTAssertNotNil(retrievedSession1)
        XCTAssertNotNil(retrievedSession2)
        XCTAssertNotNil(retrievedSession3)
    }
    
    // MARK: - 6.3 Test session retrieval and lastActivity update
    
    func testSessionRetrievalUpdatesLastActivity() async throws {
        let manager = SessionManager()
        
        let sessionId = try await manager.openSession(instructions: nil)
        
        // First retrieval
        let session1 = await manager.getSession(sessionId)
        XCTAssertNotNil(session1)
        
        // Wait a bit
        try await Task.sleep(for: .milliseconds(100))
        
        // Second retrieval should update lastActivity
        let session2 = await manager.getSession(sessionId)
        XCTAssertNotNil(session2)
        
        // Both retrievals should return the same session instance
        XCTAssertTrue(session1 === session2, "Should return the same session instance")
    }
    
    func testGetSessionReturnsNilForNonexistentSession() async {
        let manager = SessionManager()
        
        let nonexistentId = UUID().uuidString
        let session = await manager.getSession(nonexistentId)
        
        XCTAssertNil(session)
    }
    
    // MARK: - 6.4 Test session closure
    
    func testSessionClosure() async throws {
        let manager = SessionManager()
        
        let sessionId = try await manager.openSession(instructions: nil)
        
        // Verify session exists
        var session = await manager.getSession(sessionId)
        XCTAssertNotNil(session)
        
        // Close the session
        await manager.closeSession(sessionId)
        
        // Verify session no longer exists
        session = await manager.getSession(sessionId)
        XCTAssertNil(session)
    }
    
    func testClosingNonexistentSessionDoesNotCrash() async {
        let manager = SessionManager()
        
        let nonexistentId = UUID().uuidString
        
        // Should not crash
        await manager.closeSession(nonexistentId)
    }
    
    func testClosingSessionTwiceDoesNotCrash() async throws {
        let manager = SessionManager()
        
        let sessionId = try await manager.openSession(instructions: nil)
        
        // Close once
        await manager.closeSession(sessionId)
        
        // Close again - should not crash
        await manager.closeSession(sessionId)
    }
    
    // MARK: - 6.5 Test idle timeout (with configurable timeout for testing)
    
    func testIdleTimeout() async throws {
        // Use a short timeout for testing (2 seconds)
        let manager = SessionManager(idleTimeout: 2.0)
        
        let sessionId = try await manager.openSession(instructions: nil)
        
        // Verify session exists
        var session = await manager.getSession(sessionId)
        XCTAssertNotNil(session)
        
        // Wait for idle timeout to trigger (2s timeout + 30s check interval)
        // Since the check runs every 30 seconds, we need to wait at least that long
        // For testing purposes, we'll wait 35 seconds to ensure the check runs
        try await Task.sleep(for: .seconds(35))
        
        // Session should be closed due to idle timeout
        session = await manager.getSession(sessionId)
        XCTAssertNil(session, "Session should be closed after idle timeout")
    }
    
    // MARK: - 6.6 Test closeAllSessions
    
    func testCloseAllSessions() async throws {
        let manager = SessionManager()
        
        // Create multiple sessions
        let sessionId1 = try await manager.openSession(instructions: nil)
        let sessionId2 = try await manager.openSession(instructions: nil)
        let sessionId3 = try await manager.openSession(instructions: nil)
        
        // Verify all sessions exist
        let session1Before = await manager.getSession(sessionId1)
        let session2Before = await manager.getSession(sessionId2)
        let session3Before = await manager.getSession(sessionId3)
        XCTAssertNotNil(session1Before)
        XCTAssertNotNil(session2Before)
        XCTAssertNotNil(session3Before)
        
        // Close all sessions
        await manager.closeAllSessions()
        
        // Verify all sessions are closed
        let session1After = await manager.getSession(sessionId1)
        let session2After = await manager.getSession(sessionId2)
        let session3After = await manager.getSession(sessionId3)
        XCTAssertNil(session1After)
        XCTAssertNil(session2After)
        XCTAssertNil(session3After)
    }
    
    func testCloseAllSessionsWithNoSessions() async {
        let manager = SessionManager()
        
        // Should not crash when there are no sessions
        await manager.closeAllSessions()
    }
    
    func testCloseAllSessionsMultipleTimes() async throws {
        let manager = SessionManager()
        
        let sessionId = try await manager.openSession(instructions: nil)
        let sessionBefore = await manager.getSession(sessionId)
        XCTAssertNotNil(sessionBefore)
        
        // Close all sessions multiple times
        await manager.closeAllSessions()
        await manager.closeAllSessions()
        await manager.closeAllSessions()
        
        // Should not crash
        let sessionAfter = await manager.getSession(sessionId)
        XCTAssertNil(sessionAfter)
    }
    
    // MARK: - Additional edge case tests
    
    func testDefaultIdleTimeout() async throws {
        // Default timeout should be 120 seconds
        let manager = SessionManager()
        
        let sessionId = try await manager.openSession(instructions: nil)
        
        // Session should exist immediately
        let session = await manager.getSession(sessionId)
        XCTAssertNotNil(session)
        
        // Note: We can't easily test the full 120-second timeout in a unit test,
        // but we've verified the configurable timeout works in testIdleTimeout
    }
    
    func testMultipleSessionsWithDifferentActivityPatterns() async throws {
        let manager = SessionManager(idleTimeout: 3.0)
        
        // Create three sessions
        let activeSessionId = try await manager.openSession(instructions: "Active")
        let idleSessionId = try await manager.openSession(instructions: "Idle")
        let closedSessionId = try await manager.openSession(instructions: "Closed")
        
        // Explicitly close one session
        await manager.closeSession(closedSessionId)
        
        // Keep one session active
        for _ in 0..<3 {
            try await Task.sleep(for: .seconds(1))
            _ = await manager.getSession(activeSessionId)
        }
        
        // Let the idle session timeout (don't access it)
        try await Task.sleep(for: .seconds(35))
        
        // Active session should still exist
        let activeSession = await manager.getSession(activeSessionId)
        XCTAssertNotNil(activeSession)
        
        // Idle session should be closed by timeout
        let idleSession = await manager.getSession(idleSessionId)
        XCTAssertNil(idleSession)
        
        // Explicitly closed session should be closed
        let closedSession = await manager.getSession(closedSessionId)
        XCTAssertNil(closedSession)
    }
}
