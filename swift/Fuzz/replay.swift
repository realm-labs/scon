import Foundation

@main
struct Replay {
    static func main() throws {
        let target = CommandLine.arguments.dropFirst().first ?? "parse"
        let data = FileHandle.standardInput.readDataToEndOfFile()
        guard let source = String(data: data, encoding: .utf8) else {
            exit(0)
        }

        switch target {
        case "parse":
            do { _ = try Scon.parseString(source) } catch {}
        case "format-source":
            let formatted: String
            do {
                formatted = try Scon.formatSource(source)
            } catch {
                exit(0)
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
        default:
            fatalError("unknown fuzz target: \(target)")
        }
    }
}
