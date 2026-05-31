import Foundation

struct Document { let root: AstObject; let file: String? }
struct AstObject { let members: [AstMember]; let span: Span }
enum AstMember { case field(AstField), include(AstInclude), objectSpread(AstObjectSpread); var span: Span { switch self { case .field(let v): v.span; case .include(let v): v.span; case .objectSpread(let v): v.span } } }
struct AstField { let path: AstPath; let value: AstValue; let span: Span }
struct AstInclude { let path: AstString; let span: Span }
struct AstObjectSpread { let sub: AstSubstitution; let span: Span }
struct AstPath { let segments: [AstPathSegment]; let span: Span }
struct AstPathSegment { let value: String; let quoted: Bool; let span: Span }
indirect enum AstValue { case null(Span), bool(Bool, Span), number(String, Span), string(AstString), array(AstArray), object(AstObject, Span), substitution(AstSubstitution); var span: Span { switch self { case .null(let s), .bool(_, let s), .number(_, let s), .object(_, let s): s; case .string(let v): v.span; case .array(let v): v.span; case .substitution(let v): v.span } } }
struct AstString { let value: String; let raw: String; let parts: [StringPart]; let span: Span }
indirect enum StringPart { case literal(String), interpolation(AstPath, Span) }
struct AstArray { let items: [AstArrayItem]; let span: Span }
enum AstArrayItem { case value(AstValue, Span), spread(AstSubstitution, Span); var span: Span { switch self { case .value(_, let s), .spread(_, let s): s } } }
struct AstSubstitution { let path: AstPath; let span: Span }
