import Testing
@testable import JarvisListen

// MARK: - Property 2: Ring Buffer Overflow Handling

@Test("Ring buffer overflow handling - Property 2")
func testRingBufferOverflowHandling() throws {
    // Property: For any ring buffer at or near capacity, when new data is written
    // that exceeds capacity, the buffer SHALL discard the oldest data, accept the
    // new data, and maintain its capacity invariant (availableBytes ≤ capacity).
    
    let iterations = 100
    
    for _ in 0..<iterations {
        // Generate random capacity (100-1000 bytes)
        let capacity = Int.random(in: 100...1000)
        let buffer = RingBuffer(capacity: capacity)
        
        // Fill buffer to random level (50-100% full)
        let initialFill = Int.random(in: (capacity / 2)...capacity)
        let initialData = (0..<initialFill).map { _ in UInt8.random(in: 0...255) }
        _ = buffer.write(initialData)
        
        // Write data that will cause overflow (ensure initialFill + overflowSize > capacity)
        let minOverflowSize = max(1, capacity - initialFill + 1)
        let maxOverflowSize = capacity
        let overflowSize = Int.random(in: minOverflowSize...maxOverflowSize)
        let overflowData = (0..<overflowSize).map { _ in UInt8.random(in: 0...255) }
        
        let result = buffer.write(overflowData)
        
        // Verify overflow was detected
        #expect(result == false, "Should return false on overflow")
        
        // Verify capacity invariant maintained
        let available = buffer.availableData()
        #expect(available <= capacity, "Available bytes (\(available)) must not exceed capacity (\(capacity))")
        
        // Verify we can read the expected amount
        #expect(available == min(initialFill + overflowSize, capacity))
    }
}

@Test("Ring buffer handles data larger than capacity")
func testRingBufferDataLargerThanCapacity() throws {
    let capacity = 100
    let buffer = RingBuffer(capacity: capacity)
    
    // Write data larger than capacity
    let largeData = (0..<150).map { UInt8($0 % 256) }
    let result = buffer.write(largeData)
    
    // Should return false (overflow)
    #expect(result == false)
    
    // Should only keep the last 'capacity' bytes
    #expect(buffer.availableData() == capacity)
    
    // Read all data and verify it's the tail of largeData
    let readData = buffer.read(capacity)
    #expect(readData != nil)
    
    let expectedTail = Array(largeData.suffix(capacity))
    #expect(readData == expectedTail)
}

// MARK: - Property 3: Silent Channel Filling (Underflow)

@Test("Ring buffer underflow handling - Property 3")
func testRingBufferUnderflowHandling() throws {
    // Property: For any chunk generation request, when one ring buffer has
    // insufficient data (underflow), read() SHALL return nil.
    
    let iterations = 100
    
    for _ in 0..<iterations {
        let capacity = Int.random(in: 100...1000)
        let buffer = RingBuffer(capacity: capacity)
        
        // Write random amount of data (0 to capacity-1)
        let dataSize = Int.random(in: 0..<capacity)
        if dataSize > 0 {
            let data = (0..<dataSize).map { _ in UInt8.random(in: 0...255) }
            _ = buffer.write(data)
        }
        
        // Try to read more than available
        let requestSize = dataSize + Int.random(in: 1...100)
        let result = buffer.read(requestSize)
        
        // Should return nil when insufficient data
        #expect(result == nil, "Should return nil when requesting \(requestSize) bytes but only \(dataSize) available")
        
        // Available data should be unchanged
        #expect(buffer.availableData() == dataSize)
    }
}

@Test("Ring buffer returns nil on empty buffer")
func testRingBufferEmptyRead() throws {
    let buffer = RingBuffer(capacity: 100)
    
    // Try to read from empty buffer
    let result = buffer.read(10)
    #expect(result == nil)
    #expect(buffer.availableData() == 0)
}

// MARK: - Property 12: Buffer Flush Completeness

@Test("Ring buffer flush completeness - Property 12")
func testRingBufferFlushCompleteness() throws {
    // Property: For any ring buffer state at shutdown time, reading all available
    // data SHALL return all data in the buffer, leaving the buffer empty.
    
    let iterations = 100
    
    for _ in 0..<iterations {
        let capacity = Int.random(in: 100...1000)
        let buffer = RingBuffer(capacity: capacity)
        
        // Write random amount of data
        let dataSize = Int.random(in: 1...capacity)
        let originalData = (0..<dataSize).map { _ in UInt8.random(in: 0...255) }
        _ = buffer.write(originalData)
        
        let availableBefore = buffer.availableData()
        #expect(availableBefore == dataSize)
        
        // Read all available data
        let flushedData = buffer.read(availableBefore)
        
        // Verify all data was returned
        #expect(flushedData != nil)
        #expect(flushedData?.count == availableBefore)
        
        // Verify buffer is now empty
        #expect(buffer.availableData() == 0)
        
        // Verify data matches what was written
        #expect(flushedData == originalData)
    }
}

// MARK: - Additional Edge Cases

@Test("Ring buffer basic write and read")
func testRingBufferBasicWriteRead() throws {
    let buffer = RingBuffer(capacity: 100)
    
    let data: [UInt8] = [1, 2, 3, 4, 5]
    let writeResult = buffer.write(data)
    
    #expect(writeResult == true, "Should succeed without overflow")
    #expect(buffer.availableData() == 5)
    
    let readData = buffer.read(5)
    #expect(readData == data)
    #expect(buffer.availableData() == 0)
}

@Test("Ring buffer clear")
func testRingBufferClear() throws {
    let buffer = RingBuffer(capacity: 100)
    
    let data = (0..<50).map { UInt8($0) }
    _ = buffer.write(data)
    
    #expect(buffer.availableData() == 50)
    
    buffer.clear()
    
    #expect(buffer.availableData() == 0)
    
    // Should be able to write again after clear
    _ = buffer.write([1, 2, 3])
    #expect(buffer.availableData() == 3)
}

@Test("Ring buffer wraps around correctly")
func testRingBufferWrapAround() throws {
    let buffer = RingBuffer(capacity: 10)
    
    // Write 8 bytes
    _ = buffer.write([1, 2, 3, 4, 5, 6, 7, 8])
    
    // Read 5 bytes (readIndex moves to 5)
    let first = buffer.read(5)
    #expect(first == [1, 2, 3, 4, 5])
    
    // Write 7 more bytes (should wrap around)
    _ = buffer.write([9, 10, 11, 12, 13, 14, 15])
    
    // Should have 10 bytes total (3 old + 7 new)
    #expect(buffer.availableData() == 10)
    
    // Read all and verify order
    let all = buffer.read(10)
    #expect(all == [6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
}
