import XCTest
@testable import IntelligenceKit

final class ModelsTests: XCTestCase {
    
    // MARK: - 4.1 Test Request JSON decoding with snake_case fields
    
    func testRequestDecodingWithSnakeCaseFields() throws {
        let json = """
        {
            "command": "message",
            "session_id": "test-session-123",
            "prompt": "Generate tags",
            "content": "Sample content",
            "output_format": "string_list"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.command, "message")
        XCTAssertEqual(request.sessionId, "test-session-123")
        XCTAssertEqual(request.prompt, "Generate tags")
        XCTAssertEqual(request.content, "Sample content")
        XCTAssertEqual(request.outputFormat, "string_list")
    }
    
    func testRequestDecodingOpenSession() throws {
        let json = """
        {
            "command": "open-session",
            "instructions": "You are a helpful assistant"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.command, "open-session")
        XCTAssertEqual(request.instructions, "You are a helpful assistant")
        XCTAssertNil(request.sessionId)
        XCTAssertNil(request.prompt)
    }
    
    func testRequestDecodingCloseSession() throws {
        let json = """
        {
            "command": "close-session",
            "session_id": "test-session-456"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.command, "close-session")
        XCTAssertEqual(request.sessionId, "test-session-456")
    }
    
    func testRequestDecodingCheckAvailability() throws {
        let json = """
        {
            "command": "check-availability"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.command, "check-availability")
        XCTAssertNil(request.sessionId)
    }
    
    func testRequestDecodingShutdown() throws {
        let json = """
        {
            "command": "shutdown"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.command, "shutdown")
    }
    
    // MARK: - 4.2 Test Response JSON encoding (success, error, availability)
    
    func testSuccessResponseEncodingOpenSession() throws {
        let response = Response.success(
            SuccessResponse(sessionId: "new-session-789", result: nil)
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":true"))
        XCTAssertTrue(json.contains("\"session_id\":\"new-session-789\""))
        XCTAssertFalse(json.contains("\"result\""), "Nil result should not be encoded")
    }
    
    func testSuccessResponseEncodingMessage() throws {
        let response = Response.success(
            SuccessResponse(sessionId: nil, result: .text("Generated text"))
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":true"))
        XCTAssertFalse(json.contains("\"session_id\""), "Nil sessionId should not be encoded")
        XCTAssertTrue(json.contains("\"result\":\"Generated text\""))
    }
    
    func testSuccessResponseEncodingWithStringList() throws {
        let response = Response.success(
            SuccessResponse(sessionId: nil, result: .stringList(["tag1", "tag2"]))
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":true"))
        XCTAssertTrue(json.contains("\"result\":[\"tag1\",\"tag2\"]"))
        XCTAssertFalse(json.contains("stringList"), "Should not have discriminator key")
    }
    
    func testSuccessResponseEncodingCloseSession() throws {
        let response = Response.success(
            SuccessResponse(sessionId: nil, result: nil)
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":true"))
        XCTAssertFalse(json.contains("\"session_id\""), "Nil sessionId should not be encoded")
        XCTAssertFalse(json.contains("\"result\""), "Nil result should not be encoded")
    }
    
    func testErrorResponseEncoding() throws {
        let response = Response.error(
            ErrorResponse(error: "session_not_found")
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":false"))
        XCTAssertTrue(json.contains("\"error\":\"session_not_found\""))
    }
    
    func testAvailabilityResponseEncodingAvailable() throws {
        let response = Response.availability(
            AvailabilityResponse(available: true, reason: nil)
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":true"))
        XCTAssertTrue(json.contains("\"available\":true"))
        // Note: reason may or may not be present when nil (default Codable behavior)
    }
    
    func testAvailabilityResponseEncodingUnavailable() throws {
        let response = Response.availability(
            AvailabilityResponse(available: false, reason: "Apple Intelligence not enabled")
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"ok\":true"))
        XCTAssertTrue(json.contains("\"available\":false"))
        XCTAssertTrue(json.contains("\"reason\":\"Apple Intelligence not enabled\""))
    }
    
    // MARK: - 4.3 Test ResultValue encoding (string_list, text)
    
    func testResultValueEncodingStringList() throws {
        let resultValue = ResultValue.stringList(["tag1", "tag2", "tag3"])
        
        let data = try JSONEncoder().encode(resultValue)
        let json = String(data: data, encoding: .utf8)!
        
        // Should encode as a flat array without discriminator
        XCTAssertTrue(json.contains("[\"tag1\",\"tag2\",\"tag3\"]"))
        XCTAssertFalse(json.contains("stringList"), "Should not have discriminator key")
    }
    
    func testResultValueEncodingText() throws {
        let resultValue = ResultValue.text("Generated response")
        
        let data = try JSONEncoder().encode(resultValue)
        let json = String(data: data, encoding: .utf8)!
        
        // Should encode as a flat string without discriminator
        XCTAssertEqual(json, "\"Generated response\"")
        XCTAssertFalse(json.contains("{"), "Should not be wrapped in object")
    }
    
    func testResultValueDecodingStringList() throws {
        let json = "[\"item1\",\"item2\"]"
        let data = json.data(using: .utf8)!
        
        let resultValue = try JSONDecoder().decode(ResultValue.self, from: data)
        
        if case .stringList(let items) = resultValue {
            XCTAssertEqual(items, ["item1", "item2"])
        } else {
            XCTFail("Expected stringList case")
        }
    }
    
    func testResultValueDecodingText() throws {
        let json = "\"Sample text\""
        let data = json.data(using: .utf8)!
        
        let resultValue = try JSONDecoder().decode(ResultValue.self, from: data)
        
        if case .text(let text) = resultValue {
            XCTAssertEqual(text, "Sample text")
        } else {
            XCTFail("Expected text case")
        }
    }
    
    // MARK: - 4.4 Test CodingKeys mapping (sessionId ↔ session_id, outputFormat ↔ output_format)
    
    func testCodingKeysSessionIdMapping() throws {
        // Encode with sessionId, should produce session_id in JSON
        let response = Response.success(
            SuccessResponse(sessionId: "test-123", result: nil)
        )
        
        let data = try JSONEncoder().encode(response)
        let json = String(data: data, encoding: .utf8)!
        
        XCTAssertTrue(json.contains("\"session_id\":\"test-123\""))
        XCTAssertFalse(json.contains("\"sessionId\""), "Should use snake_case in JSON")
    }
    
    func testCodingKeysOutputFormatMapping() throws {
        // Decode with output_format, should map to outputFormat
        let json = """
        {
            "command": "message",
            "session_id": "test-session",
            "prompt": "Test",
            "content": "Content",
            "output_format": "text"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.outputFormat, "text")
    }
    
    func testCodingKeysRoundTrip() throws {
        // Test that we can decode snake_case and encode back to snake_case
        let originalJson = """
        {
            "command": "message",
            "session_id": "session-abc",
            "output_format": "string_list",
            "prompt": "Generate",
            "content": "Data"
        }
        """
        
        let data = originalJson.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        // Verify decoded values
        XCTAssertEqual(request.sessionId, "session-abc")
        XCTAssertEqual(request.outputFormat, "string_list")
        
        // Note: We can't re-encode Request since it doesn't have an encode method,
        // but we've verified the decoding works correctly
    }
    
    // MARK: - Additional edge case tests
    
    func testCommandEnumRawValues() {
        XCTAssertEqual(Command.openSession.rawValue, "open-session")
        XCTAssertEqual(Command.message.rawValue, "message")
        XCTAssertEqual(Command.closeSession.rawValue, "close-session")
        XCTAssertEqual(Command.checkAvailability.rawValue, "check-availability")
        XCTAssertEqual(Command.shutdown.rawValue, "shutdown")
    }
    
    func testInvalidCommandDecoding() throws {
        let json = """
        {
            "command": "invalid-command"
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        // Request should decode successfully, but Command enum won't match
        XCTAssertEqual(request.command, "invalid-command")
        XCTAssertNil(Command(rawValue: request.command))
    }
    
    func testEmptyStringFields() throws {
        let json = """
        {
            "command": "message",
            "session_id": "",
            "prompt": "",
            "content": "",
            "output_format": ""
        }
        """
        
        let data = json.data(using: .utf8)!
        let request = try JSONDecoder().decode(Request.self, from: data)
        
        XCTAssertEqual(request.sessionId, "")
        XCTAssertEqual(request.prompt, "")
        XCTAssertEqual(request.content, "")
        XCTAssertEqual(request.outputFormat, "")
    }
}
