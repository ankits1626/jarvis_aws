import Testing
@testable import JarvisListen

// MARK: - Property 7: Sample Rate Validation

@Test("Sample rate validation - Property 7")
func testSampleRateValidation() throws {
    // Property: For any integer value provided as --sample-rate argument,
    // the argument parser SHALL accept it if and only if it is in the set
    // {8000, 16000, 24000, 44100, 48000}, and SHALL reject all other values.
    
    let validRates = [8000, 16000, 24000, 44100, 48000]
    let iterations = 100
    
    // Test valid rates
    for rate in validRates {
        let args = ["JarvisListen", "--sample-rate", "\(rate)"]
        let result = try ArgumentParser.parse(args)
        
        if case .capture(let config) = result.action {
            #expect(config.sampleRate == rate)
        } else {
            Issue.record("Expected capture action for valid rate \(rate)")
        }
    }
    
    // Test invalid rates (random integers outside valid set)
    for _ in 0..<iterations {
        var invalidRate: Int
        repeat {
            invalidRate = Int.random(in: 1000...100000)
        } while validRates.contains(invalidRate)
        
        let args = ["JarvisListen", "--sample-rate", "\(invalidRate)"]
        
        do {
            _ = try ArgumentParser.parse(args)
            Issue.record("Should reject invalid sample rate \(invalidRate)")
        } catch let error as ArgumentParser.ParseError {
            if case .invalidSampleRate(let value) = error {
                #expect(value == "\(invalidRate)")
                // Verify error message lists valid values
                let errorMsg = error.description
                #expect(errorMsg.contains("8000"))
                #expect(errorMsg.contains("16000"))
                #expect(errorMsg.contains("24000"))
                #expect(errorMsg.contains("44100"))
                #expect(errorMsg.contains("48000"))
            } else {
                Issue.record("Expected invalidSampleRate error, got \(error)")
            }
        } catch {
            Issue.record("Expected ParseError, got \(error)")
        }
    }
    
    // Test non-numeric sample rate
    let args = ["JarvisListen", "--sample-rate", "invalid"]
    do {
        _ = try ArgumentParser.parse(args)
        Issue.record("Should reject non-numeric sample rate")
    } catch let error as ArgumentParser.ParseError {
        if case .invalidSampleRate = error {
            // Expected
        } else {
            Issue.record("Expected invalidSampleRate error")
        }
    }
}

// MARK: - Property 8: Device ID Argument Parsing

@Test("Device ID argument parsing - Property 8")
func testDeviceIDArgumentParsing() throws {
    // Property: For any string provided as --mic-device argument,
    // the argument parser SHALL store that string in the configuration's
    // microphoneDeviceID field without modification.
    
    let testDeviceIDs = [
        "BuiltInMicrophoneDevice",
        "USB-Audio-Device-123",
        "device with spaces",
        "device-with-special-chars!@#$",
        "",
        "123456",
        "a",
        String(repeating: "x", count: 100)
    ]
    
    for deviceID in testDeviceIDs {
        let args = ["JarvisListen", "--mic-device", deviceID]
        let result = try ArgumentParser.parse(args)
        
        if case .capture(let config) = result.action {
            #expect(config.microphoneDeviceID == deviceID,
                   "Device ID should be stored exactly as provided: '\(deviceID)'")
        } else {
            Issue.record("Expected capture action")
        }
    }
    
    // Test random strings
    for _ in 0..<50 {
        let randomLength = Int.random(in: 1...50)
        let randomID = (0..<randomLength).map { _ in
            String(UnicodeScalar(Int.random(in: 32...126))!)
        }.joined()
        
        let args = ["JarvisListen", "--mic-device", randomID]
        let result = try ArgumentParser.parse(args)
        
        if case .capture(let config) = result.action {
            #expect(config.microphoneDeviceID == randomID)
        } else {
            Issue.record("Expected capture action")
        }
    }
}

// MARK: - Property 13: Invalid Flag Rejection

@Test("Invalid flag rejection - Property 13")
func testInvalidFlagRejection() throws {
    // Property: For any command-line argument that is not a recognized flag or value,
    // the argument parser SHALL reject it with an error message and non-zero exit code.
    
    let invalidFlags = [
        "--invalid",
        "--xyz",
        "-z",
        "--unknown-flag",
        "--MONO",  // case sensitive
        "--sample",
        "--device",
        "random-arg",
        "--help-me",
        "--list"
    ]
    
    for flag in invalidFlags {
        let args = ["JarvisListen", flag]
        
        do {
            _ = try ArgumentParser.parse(args)
            Issue.record("Should reject invalid flag '\(flag)'")
        } catch let error as ArgumentParser.ParseError {
            if case .invalidFlag(let rejectedFlag) = error {
                #expect(rejectedFlag == flag)
                // Verify error message suggests --help
                let errorMsg = error.description
                #expect(errorMsg.contains("--help"))
            } else {
                Issue.record("Expected invalidFlag error for '\(flag)'")
            }
        } catch {
            Issue.record("Expected ParseError, got \(error)")
        }
    }
    
    // Test random invalid flags
    for _ in 0..<50 {
        let randomFlag = "--random\(Int.random(in: 1000...9999))"
        let args = ["JarvisListen", randomFlag]
        
        do {
            _ = try ArgumentParser.parse(args)
            Issue.record("Should reject random flag '\(randomFlag)'")
        } catch let error as ArgumentParser.ParseError {
            if case .invalidFlag = error {
                // Expected
            } else {
                Issue.record("Expected invalidFlag error")
            }
        }
    }
}

// MARK: - Property 18: Default Configuration Values

@Test("Default configuration values - Property 18")
func testDefaultConfigurationValues() throws {
    // Property: When invoked without arguments, the system SHALL use default values:
    // 16000Hz sample rate, stereo output, nil microphone device.
    
    let args = ["JarvisListen"]
    let result = try ArgumentParser.parse(args)
    
    if case .capture(let config) = result.action {
        #expect(config.sampleRate == 16000, "Default sample rate should be 16000")
        #expect(config.outputMono == false, "Default should be stereo (not mono)")
        #expect(config.microphoneDeviceID == nil, "Default device should be nil (system default)")
    } else {
        Issue.record("Expected capture action with default configuration")
    }
}

// MARK: - Property 19: Mono Flag Configuration

@Test("Mono flag configuration - Property 19")
func testMonoFlagConfiguration() throws {
    // Property: When --mono flag is provided, the configuration SHALL have outputMono=true.
    
    let args = ["JarvisListen", "--mono"]
    let result = try ArgumentParser.parse(args)
    
    if case .capture(let config) = result.action {
        #expect(config.outputMono == true, "--mono flag should set outputMono to true")
    } else {
        Issue.record("Expected capture action")
    }
    
    // Test --mono with other flags
    let args2 = ["JarvisListen", "--mono", "--sample-rate", "48000"]
    let result2 = try ArgumentParser.parse(args2)
    
    if case .capture(let config) = result2.action {
        #expect(config.outputMono == true)
        #expect(config.sampleRate == 48000)
    } else {
        Issue.record("Expected capture action")
    }
}

// MARK: - Property 20: Sample Rate Flag Configuration

@Test("Sample rate flag configuration - Property 20")
func testSampleRateFlagConfiguration() throws {
    // Property: When --sample-rate flag is provided with a valid value,
    // the configuration SHALL use that sample rate.
    
    let testRates = [8000, 16000, 24000, 44100, 48000]
    
    for rate in testRates {
        let args = ["JarvisListen", "--sample-rate", "\(rate)"]
        let result = try ArgumentParser.parse(args)
        
        if case .capture(let config) = result.action {
            #expect(config.sampleRate == rate,
                   "--sample-rate \(rate) should set sampleRate to \(rate)")
        } else {
            Issue.record("Expected capture action")
        }
    }
}

// MARK: - Additional Edge Cases

@Test("Help flag returns showHelp action")
func testHelpFlag() throws {
    let args = ["JarvisListen", "--help"]
    let result = try ArgumentParser.parse(args)
    
    if case .showHelp = result.action {
        // Expected
    } else {
        Issue.record("Expected showHelp action")
    }
}

@Test("List devices flag returns listDevices action")
func testListDevicesFlag() throws {
    let args = ["JarvisListen", "--list-devices"]
    let result = try ArgumentParser.parse(args)
    
    if case .listDevices = result.action {
        // Expected
    } else {
        Issue.record("Expected listDevices action")
    }
}

@Test("Missing sample rate value throws error")
func testMissingSampleRateValue() throws {
    let args = ["JarvisListen", "--sample-rate"]
    
    do {
        _ = try ArgumentParser.parse(args)
        Issue.record("Should throw error for missing sample rate value")
    } catch let error as ArgumentParser.ParseError {
        if case .missingSampleRateValue = error {
            // Expected
        } else {
            Issue.record("Expected missingSampleRateValue error")
        }
    }
}

@Test("Missing mic device value throws error")
func testMissingMicDeviceValue() throws {
    let args = ["JarvisListen", "--mic-device"]
    
    do {
        _ = try ArgumentParser.parse(args)
        Issue.record("Should throw error for missing mic device value")
    } catch let error as ArgumentParser.ParseError {
        if case .missingMicDeviceValue = error {
            // Expected
        } else {
            Issue.record("Expected missingMicDeviceValue error")
        }
    }
}

@Test("Multiple flags can be combined")
func testMultipleFlagsCombined() throws {
    let args = ["JarvisListen", "--mono", "--sample-rate", "48000", "--mic-device", "test-device"]
    let result = try ArgumentParser.parse(args)
    
    if case .capture(let config) = result.action {
        #expect(config.outputMono == true)
        #expect(config.sampleRate == 48000)
        #expect(config.microphoneDeviceID == "test-device")
    } else {
        Issue.record("Expected capture action")
    }
}
