import Foundation

@_cdecl("LLVMFuzzerTestOneInput")
public func LLVMFuzzerTestOneInput(_ data: UnsafePointer<UInt8>, _ size: Int) -> Int32 {
    let bytes = UnsafeBufferPointer(start: data, count: size)
    guard let source = String(bytes: bytes, encoding: .utf8) else { return 0 }

    let formatted: String
    do {
        formatted = try Scon.formatSource(source)
    } catch {
        return 0
    }

    if Scon.analyzeSource(formatted).parsed == nil {
        fatalError("formatted source does not parse")
    }

    do {
        let original = try Scon.parseString(source)
        let roundTrip = try Scon.parseString(formatted)
        if try Scon.formatValue(original) != Scon.formatValue(roundTrip) {
            fatalError("formatted source changed resolved value")
        }
    } catch {}

    return 0
}
