import Foundation
import CoreMedia
import AVFoundation

// MARK: - Shared Test Helpers

/// Helper to create a CMSampleBuffer with specific byte data
func createTestSampleBufferWithData(
    data: [UInt8],
    sampleRate: Double,
    channels: UInt32
) throws -> CMSampleBuffer {
    let bytesPerSample: UInt32 = 2  // Int16
    let bytesPerFrame = channels * bytesPerSample
    let frameCount = data.count / Int(bytesPerFrame)
    
    // Create audio format description
    var audioFormat = AudioStreamBasicDescription(
        mSampleRate: sampleRate,
        mFormatID: kAudioFormatLinearPCM,
        mFormatFlags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
        mBytesPerPacket: bytesPerFrame,
        mFramesPerPacket: 1,
        mBytesPerFrame: bytesPerFrame,
        mChannelsPerFrame: channels,
        mBitsPerChannel: 16,
        mReserved: 0
    )
    
    var formatDescription: CMAudioFormatDescription?
    let formatStatus = CMAudioFormatDescriptionCreate(
        allocator: kCFAllocatorDefault,
        asbd: &audioFormat,
        layoutSize: 0,
        layout: nil,
        magicCookieSize: 0,
        magicCookie: nil,
        extensions: nil,
        formatDescriptionOut: &formatDescription
    )
    
    guard formatStatus == noErr, let formatDesc = formatDescription else {
        throw TestError.failedToCreateFormatDescription
    }
    
    // Create CMBlockBuffer with our data
    var blockBuffer: CMBlockBuffer?
    let blockStatus = CMBlockBufferCreateWithMemoryBlock(
        allocator: kCFAllocatorDefault,
        memoryBlock: nil,
        blockLength: data.count,
        blockAllocator: kCFAllocatorDefault,
        customBlockSource: nil,
        offsetToData: 0,
        dataLength: data.count,
        flags: 0,
        blockBufferOut: &blockBuffer
    )
    
    guard blockStatus == noErr, let block = blockBuffer else {
        throw TestError.failedToCreateBlockBuffer
    }
    
    // Copy our data to block buffer
    let copyStatus = CMBlockBufferReplaceDataBytes(
        with: data,
        blockBuffer: block,
        offsetIntoDestination: 0,
        dataLength: data.count
    )
    
    guard copyStatus == noErr else {
        throw TestError.failedToCopyData
    }
    
    // Create CMSampleBuffer
    var sampleBuffer: CMSampleBuffer?
    let sampleStatus = CMAudioSampleBufferCreateReadyWithPacketDescriptions(
        allocator: kCFAllocatorDefault,
        dataBuffer: block,
        formatDescription: formatDesc,
        sampleCount: frameCount,
        presentationTimeStamp: CMTime.zero,
        packetDescriptions: nil,
        sampleBufferOut: &sampleBuffer
    )
    
    guard sampleStatus == noErr, let buffer = sampleBuffer else {
        throw TestError.failedToCreateSampleBuffer
    }
    
    return buffer
}

enum TestError: Error {
    case failedToCreateFormatDescription
    case failedToCreateBlockBuffer
    case failedToCopyData
    case failedToCreateSampleBuffer
}
