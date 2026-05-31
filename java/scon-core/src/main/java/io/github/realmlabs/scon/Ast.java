package io.github.realmlabs.scon;

import java.util.List;

final class Ast {
    record Document(ObjectNode root, String file) {}
    record ObjectNode(List<Member> members, Span span) {}
    sealed interface Member permits Field, Include, ObjectSpread { Span span(); }
    record Field(PathNode path, ValueNode value, Span span) implements Member {}
    record Include(StringNode path, Span span) implements Member {}
    record ObjectSpread(SubstitutionNode sub, Span span) implements Member {}
    record PathNode(List<PathSegment> segments, Span span) {}
    record PathSegment(String value, boolean quoted, Span span) {}
    sealed interface ValueNode permits NullNode, BoolNode, NumberNode, StringNode, ArrayNode, ObjectValueNode, SubstitutionNode {
        Span span();
    }
    record NullNode(Span span) implements ValueNode {}
    record BoolNode(boolean value, Span span) implements ValueNode {}
    record NumberNode(String raw, Span span) implements ValueNode {}
    record StringNode(String value, String raw, List<StringPart> parts, Span span) implements ValueNode {}
    record ArrayNode(List<ArrayItem> items, Span span) implements ValueNode {}
    record ObjectValueNode(ObjectNode object, Span span) implements ValueNode {}
    record SubstitutionNode(PathNode path, Span span) implements ValueNode {}
    sealed interface ArrayItem permits ArrayValue, ArraySpread { Span span(); }
    record ArrayValue(ValueNode value, Span span) implements ArrayItem {}
    record ArraySpread(SubstitutionNode sub, Span span) implements ArrayItem {}
    sealed interface StringPart permits StringLiteral, StringInterpolation {}
    record StringLiteral(String value) implements StringPart {}
    record StringInterpolation(PathNode path, Span span) implements StringPart {}

    private Ast() {}
}
