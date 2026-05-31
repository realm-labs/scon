import Foundation
import Testing
@testable import SconCore

@Suite struct SconCoreTests {
    @Test func conformance() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let conformance = root.appendingPathComponent("tests/conformance")
        let data = try Data(contentsOf: conformance.appendingPathComponent("manifest.json"))
        let manifest = try JSONSerialization.jsonObject(with: data) as! [String: Any]
        let cases = manifest["cases"] as! [[String: Any]]
        for item in cases {
            let entry = conformance.appendingPathComponent(item["entry"] as! String).path
            let expectedURL = conformance.appendingPathComponent(item["expected"] as! String)
            if item["kind"] as! String == "valid" {
                let expected = try JSONSerialization.jsonObject(with: Data(contentsOf: expectedURL))
                let value: SconValue
                do {
                    value = try Scon.parseFile(entry)
                } catch {
                    Issue.record("unexpected error for \(item["id"]!): \(error)")
                    continue
                }
                #expect(jsonEqual(expected, Scon.plain(value)), "\(item["id"]!)")
                #expect(jsonEqual(expected, Scon.plain(try Scon.parseString(try Scon.formatValue(value)))), "\(item["id"]!)")
            } else {
                let expected = try JSONSerialization.jsonObject(with: Data(contentsOf: expectedURL)) as! [String: Any]
                do {
                    _ = try Scon.parseFile(entry)
                    Issue.record("expected error for \(item["id"]!)")
                } catch let error as SconError {
                    #expect(error.code.rawValue == expected["code"] as! String, "\(item["id"]!)")
                }
            }
        }
    }

    @Test func codableRoundTrip() throws {
        struct Config: Codable, Equatable { let name: String; let port: Int; let tags: [String]; let mode: Mode }
        enum Mode: String, Codable { case fast, slow }
        let cfg = try Scon.decode("name = \"demo\"\nport = 8080\ntags = [\"a\", \"b\"]\nmode = \"fast\"", as: Config.self)
        #expect(cfg == Config(name: "demo", port: 8080, tags: ["a", "b"], mode: .fast))
        #expect(try Scon.decode(try Scon.encode(cfg), as: Config.self) == cfg)
    }
}

private func jsonEqual(_ a: Any, _ b: Any) -> Bool {
    if let ad = number(a), let bd = number(b) { return abs(ad - bd) < 0.0000001 }
    if a is NSNull, b is NSNull { return true }
    if let av = a as? Bool, let bv = b as? Bool { return av == bv }
    if let av = a as? String, let bv = b as? String { return av == bv }
    if let aa = a as? [Any], let ba = b as? [Any] { return aa.count == ba.count && zip(aa, ba).allSatisfy(jsonEqual) }
    if let ao = a as? [String: Any], let bo = b as? [String: Any] { return ao.count == bo.count && ao.allSatisfy { key, value in bo[key].map { jsonEqual(value, $0) } ?? false } }
    return false
}

private func number(_ value: Any) -> Double? {
    if let value = value as? Int { return Double(value) }
    if let value = value as? Int64 { return Double(value) }
    if let value = value as? UInt64 { return Double(value) }
    if let value = value as? Double { return value }
    if let value = value as? NSNumber { return value.doubleValue }
    return nil
}
