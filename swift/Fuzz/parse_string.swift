import Foundation

@_cdecl("LLVMFuzzerTestOneInput")
public func LLVMFuzzerTestOneInput(_ data: UnsafePointer<UInt8>, _ size: Int) -> Int32 {
    let bytes = UnsafeBufferPointer(start: data, count: size)
    guard let source = String(bytes: bytes, encoding: .utf8) else { return 0 }
    do { _ = try Scon.parseString(source) } catch {}
    return 0
}
