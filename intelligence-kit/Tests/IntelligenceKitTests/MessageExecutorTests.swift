import XCTest
import FoundationModels
@testable import IntelligenceKit

final class MessageExecutorTests: XCTestCase {
    
    // MARK: - Test Content Truncation
    
    func testContentTruncationAt10000Characters() {
        let executor = MessageExecutor()
        let longContent = String(repeating: "a", count: 15_000)
        
        let truncated = executor.truncateContent(longContent)
        
        XCTAssertEqual(truncated.count, 10_000)
        XCTAssertLessThan(truncated.count, longContent.count)
    }
    
    func testContentTruncationLeavesShortContentUnchanged() {
        let executor = MessageExecutor()
        let shortContent = "This is short content"
        
        let result = executor.truncateContent(shortContent)
        
        XCTAssertEqual(result, shortContent)
        XCTAssertEqual(result.count, shortContent.count)
    }
    
    func testContentTruncationAtExactly10000Characters() {
        let executor = MessageExecutor()
        let exactContent = String(repeating: "x", count: 10_000)
        
        let result = executor.truncateContent(exactContent)
        
        XCTAssertEqual(result.count, 10_000)
        XCTAssertEqual(result, exactContent)
    }
    
    // MARK: - Test Prompt Construction
    
    func testPromptConstructionIncludesBothPromptAndContent() {
        let executor = MessageExecutor()
        let prompt = "Generate tags"
        let content = "This is test content"
        
        let result = executor.constructPrompt(prompt: prompt, content: content)
        
        XCTAssertTrue(result.contains(prompt))
        XCTAssertTrue(result.contains("Content:"))
        XCTAssertTrue(result.contains(content))
        
        // Verify exact format
        let expected = """
\(prompt)

Content:
\(content)
"""
        XCTAssertEqual(result, expected)
    }
    
    func testPromptConstructionWithEmptyPrompt() {
        let executor = MessageExecutor()
        let content = "Some content"
        
        let result = executor.constructPrompt(prompt: "", content: content)
        
        XCTAssertTrue(result.contains("Content:"))
        XCTAssertTrue(result.contains(content))
    }
    
    func testPromptConstructionWithMultilineContent() {
        let executor = MessageExecutor()
        let prompt = "Summarize"
        let content = "Line 1\nLine 2\nLine 3"
        
        let result = executor.constructPrompt(prompt: prompt, content: content)
        
        XCTAssertTrue(result.contains(prompt))
        XCTAssertTrue(result.contains("Line 1"))
        XCTAssertTrue(result.contains("Line 2"))
        XCTAssertTrue(result.contains("Line 3"))
    }
    
    // MARK: - Test Unknown Output Format Error
    
    func testUnknownOutputFormatThrowsError() async throws {
        let session = LanguageModelSession(instructions: "test")
        let executor = MessageExecutor()
        
        do {
            _ = try await executor.execute(
                session: session,
                prompt: "test",
                content: "test",
                outputFormat: "json"
            )
            XCTFail("Should have thrown unknownOutputFormat")
        } catch ExecutionError.unknownOutputFormat(let format) {
            XCTAssertEqual(format, "json")
        } catch {
            XCTFail("Wrong error type: \(error)")
        }
    }
    
    func testUnknownOutputFormatXML() async throws {
        let session = LanguageModelSession(instructions: "test")
        let executor = MessageExecutor()
        
        do {
            _ = try await executor.execute(
                session: session,
                prompt: "test",
                content: "test",
                outputFormat: "xml"
            )
            XCTFail("Should have thrown unknownOutputFormat")
        } catch ExecutionError.unknownOutputFormat(let format) {
            XCTAssertEqual(format, "xml")
        } catch {
            XCTFail("Wrong error type: \(error)")
        }
    }
    
    func testUnknownOutputFormatCSV() async throws {
        let session = LanguageModelSession(instructions: "test")
        let executor = MessageExecutor()
        
        do {
            _ = try await executor.execute(
                session: session,
                prompt: "test",
                content: "test",
                outputFormat: "csv"
            )
            XCTFail("Should have thrown unknownOutputFormat")
        } catch ExecutionError.unknownOutputFormat(let format) {
            XCTAssertEqual(format, "csv")
        } catch {
            XCTFail("Wrong error type: \(error)")
        }
    }
    
    // MARK: - Test Error Cases
    
    func testExecutionErrorTypes() {
        // Verify all ExecutionError cases exist
        let unknownFormatError = ExecutionError.unknownOutputFormat("invalid")
        let guardrailError = ExecutionError.guardrailBlocked
        let unavailableError = ExecutionError.modelUnavailable
        
        // Verify error cases can be created
        switch unknownFormatError {
        case .unknownOutputFormat(let format):
            XCTAssertEqual(format, "invalid")
        default:
            XCTFail("Wrong error type")
        }
        
        switch guardrailError {
        case .guardrailBlocked:
            break // Expected
        default:
            XCTFail("Wrong error type")
        }
        
        switch unavailableError {
        case .modelUnavailable:
            break // Expected
        default:
            XCTFail("Wrong error type")
        }
    }
    
    // MARK: - Integration Test Documentation
    
    func testMessageExecutorRequiresRealModelForFullTesting() {
        // This test documents that full MessageExecutor testing with
        // string_list and text output formats requires a real
        // LanguageModelSession with Apple Intelligence enabled.
        // 
        // Full integration tests with actual model execution should be
        // performed in Phase 9 (Integration Testing) when running on
        // macOS 15+ with Apple Intelligence enabled.
        //
        // Expected behaviors to test in integration:
        // 1. string_list format returns ResultValue.stringList([String])
        // 2. text format returns ResultValue.text(String)
        // 3. Guardrail blocks throw ExecutionError.guardrailBlocked
        // 4. Model unavailable throws ExecutionError.modelUnavailable
        // 5. Content is truncated to 10,000 characters before sending to model
        // 6. Prompt includes both user prompt and content
        
        XCTAssertTrue(true, "Integration tests deferred to Phase 9")
    }
}
