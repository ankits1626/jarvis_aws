import XCTest
import FoundationModels
@testable import IntelligenceKit

final class CommandRouterTests: XCTestCase {
    
    var sessionManager: SessionManager!
    var messageExecutor: MessageExecutor!
    var availabilityChecker: AvailabilityChecker!
    var router: CommandRouter!
    
    override func setUp() async throws {
        sessionManager = SessionManager()
        messageExecutor = MessageExecutor()
        availabilityChecker = AvailabilityChecker()
        router = CommandRouter(
            sessionManager: sessionManager,
            messageExecutor: messageExecutor,
            availabilityChecker: availabilityChecker
        )
    }
    
    // MARK: - Test Unknown Command
    
    func testUnknownCommandReturnsError() async {
        let request = Request(
            command: "invalid-command",
            sessionId: nil,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "unknown_command")
        default:
            XCTFail("Expected error response")
        }
    }
    
    // MARK: - Test Open Session
    
    func testOpenSessionSuccess() async {
        let request = Request(
            command: "open-session",
            sessionId: nil,
            instructions: "You are a helpful assistant",
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .success(let successResponse):
            XCTAssertTrue(successResponse.ok)
            XCTAssertNotNil(successResponse.sessionId)
            XCTAssertNil(successResponse.result)
        default:
            XCTFail("Expected success response")
        }
    }
    
    func testOpenSessionWithNilInstructions() async {
        let request = Request(
            command: "open-session",
            sessionId: nil,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .success(let successResponse):
            XCTAssertTrue(successResponse.ok)
            XCTAssertNotNil(successResponse.sessionId)
        default:
            XCTFail("Expected success response")
        }
    }
    
    // MARK: - Test Message
    
    func testMessageWithMissingSessionId() async {
        let request = Request(
            command: "message",
            sessionId: nil,
            instructions: nil,
            prompt: "test",
            content: "test",
            outputFormat: "text"
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "session_id_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testMessageWithEmptyPrompt() async {
        let request = Request(
            command: "message",
            sessionId: "test-session",
            instructions: nil,
            prompt: "",
            content: "test",
            outputFormat: "text"
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "prompt_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testMessageWithMissingPrompt() async {
        let request = Request(
            command: "message",
            sessionId: "test-session",
            instructions: nil,
            prompt: nil,
            content: "test",
            outputFormat: "text"
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "prompt_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testMessageWithEmptyContent() async {
        let request = Request(
            command: "message",
            sessionId: "test-session",
            instructions: nil,
            prompt: "test",
            content: "",
            outputFormat: "text"
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "content_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testMessageWithMissingContent() async {
        let request = Request(
            command: "message",
            sessionId: "test-session",
            instructions: nil,
            prompt: "test",
            content: nil,
            outputFormat: "text"
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "content_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testMessageWithMissingOutputFormat() async {
        let request = Request(
            command: "message",
            sessionId: "test-session",
            instructions: nil,
            prompt: "test",
            content: "test",
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "output_format_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testMessageWithNonexistentSession() async {
        let request = Request(
            command: "message",
            sessionId: "nonexistent-session",
            instructions: nil,
            prompt: "test",
            content: "test",
            outputFormat: "text"
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "session_not_found")
        default:
            XCTFail("Expected error response")
        }
    }
    
    // MARK: - Test Close Session
    
    func testCloseSessionWithMissingSessionId() async {
        let request = Request(
            command: "close-session",
            sessionId: nil,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .error(let errorResponse):
            XCTAssertEqual(errorResponse.error, "session_id_required")
        default:
            XCTFail("Expected error response")
        }
    }
    
    func testCloseSessionSuccess() async {
        // First open a session
        let openRequest = Request(
            command: "open-session",
            sessionId: nil,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let openResponse = await router.route(openRequest)
        
        guard case .success(let successResponse) = openResponse,
              let sessionId = successResponse.sessionId else {
            XCTFail("Failed to open session")
            return
        }
        
        // Now close it
        let closeRequest = Request(
            command: "close-session",
            sessionId: sessionId,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let closeResponse = await router.route(closeRequest)
        
        switch closeResponse {
        case .success(let successResponse):
            XCTAssertTrue(successResponse.ok)
            XCTAssertNil(successResponse.sessionId)
            XCTAssertNil(successResponse.result)
        default:
            XCTFail("Expected success response")
        }
    }
    
    func testCloseSessionWithNonexistentSession() async {
        // Closing a nonexistent session should still return success
        let request = Request(
            command: "close-session",
            sessionId: "nonexistent-session",
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .success(let successResponse):
            XCTAssertTrue(successResponse.ok)
        default:
            XCTFail("Expected success response")
        }
    }
    
    // MARK: - Test Check Availability
    
    func testCheckAvailabilityReturnsCorrectStructure() async {
        let request = Request(
            command: "check-availability",
            sessionId: nil,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .availability(let availabilityResponse):
            XCTAssertTrue(availabilityResponse.ok)
            // available can be true or false depending on environment
            if !availabilityResponse.available {
                XCTAssertNotNil(availabilityResponse.reason)
            }
        default:
            XCTFail("Expected availability response")
        }
    }
    
    // MARK: - Test Shutdown
    
    func testShutdownClosesAllSessions() async throws {
        // Open multiple sessions
        let session1 = try await sessionManager.openSession(instructions: nil)
        let session2 = try await sessionManager.openSession(instructions: nil)
        let session3 = try await sessionManager.openSession(instructions: nil)
        
        // Verify sessions exist
        let s1 = await sessionManager.getSession(session1)
        let s2 = await sessionManager.getSession(session2)
        let s3 = await sessionManager.getSession(session3)
        XCTAssertNotNil(s1)
        XCTAssertNotNil(s2)
        XCTAssertNotNil(s3)
        
        // Send shutdown command
        let request = Request(
            command: "shutdown",
            sessionId: nil,
            instructions: nil,
            prompt: nil,
            content: nil,
            outputFormat: nil
        )
        
        let response = await router.route(request)
        
        switch response {
        case .success(let successResponse):
            XCTAssertTrue(successResponse.ok)
        default:
            XCTFail("Expected success response")
        }
        
        // Verify all sessions are closed
        let s1After = await sessionManager.getSession(session1)
        let s2After = await sessionManager.getSession(session2)
        let s3After = await sessionManager.getSession(session3)
        XCTAssertNil(s1After)
        XCTAssertNil(s2After)
        XCTAssertNil(s3After)
    }
}
