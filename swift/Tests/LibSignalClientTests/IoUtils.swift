//
// Copyright 2024 Signal Messenger, LLC.
// SPDX-License-Identifier: AGPL-3.0-only
//

import Foundation
import LibSignalClient

internal struct TestIoError: Error {}

public class ErrorInputStream: SignalInputStream {
    public func read(into buffer: UnsafeMutableRawBufferPointer) throws -> Int {
        throw TestIoError()
    }

    public func skip(by amount: UInt64) throws {
        throw TestIoError()
    }
}

public class ThrowsAfterInputStream: SignalInputStream {
    public init(inner: SignalInputStream, readBeforeThrow: UInt64) {
        self.inner = inner
        self.readBeforeThrow = readBeforeThrow
    }

    public func read(into buffer: UnsafeMutableRawBufferPointer) throws -> Int {
        if readBeforeThrow == 0 {
            throw TestIoError()
        }

        var target = buffer
        if buffer.count > readBeforeThrow {
            target = UnsafeMutableRawBufferPointer(rebasing: buffer[..<Int(readBeforeThrow)])
        }

        let read = try inner.read(into: target)
        if read > 0 {
            readBeforeThrow -= UInt64(read)
        }
        return read
    }

    public func skip(by amount: UInt64) throws {
        if readBeforeThrow < amount {
            readBeforeThrow = 0
            throw TestIoError()
        }

        try inner.skip(by: amount)
        readBeforeThrow -= amount
    }

    private var inner: SignalInputStream
    private var readBeforeThrow: UInt64
}

func readResource(forName name: String) -> Data {
    try! Data(
        contentsOf: URL(fileURLWithPath: #file)
            .deletingLastPathComponent()
            .appendingPathComponent("Resources")
            .appendingPathComponent(name))
}
