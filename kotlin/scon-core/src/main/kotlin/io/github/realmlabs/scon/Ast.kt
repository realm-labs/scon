package io.github.realmlabs.scon

import java.nio.file.Path

public data class SourceSpan(
    val start: Int,
    val end: Int,
) {
    init {
        require(start <= end) { "source span start must be <= end" }
    }
}

public data class ParsedDocument(
    val sourceName: String,
    val sourcePath: Path?,
    val root: AstObject,
    val tokens: List<SconToken>,
)

public sealed interface AstNode {
    public val span: SourceSpan
}

public data class AstObject(
    val members: List<AstObjectMember>,
    override val span: SourceSpan,
    val explicitBraces: Boolean,
) : AstNode

public sealed interface AstObjectMember : AstNode

public data class AstField(
    val path: AstPath,
    val value: AstValue,
    override val span: SourceSpan,
) : AstObjectMember

public data class AstInclude(
    val path: AstString,
    override val span: SourceSpan,
) : AstObjectMember

public data class AstObjectSpread(
    val substitution: AstSubstitution,
    override val span: SourceSpan,
) : AstObjectMember

public data class AstPath(
    val segments: List<AstPathSegment>,
    override val span: SourceSpan,
) : AstNode

public data class AstPathSegment(
    val value: String,
    val quoted: Boolean,
    override val span: SourceSpan,
) : AstNode

public sealed interface AstValue : AstNode

public data class AstNull(override val span: SourceSpan) : AstValue
public data class AstBool(val value: Boolean, override val span: SourceSpan) : AstValue
public data class AstNumber(val raw: String, override val span: SourceSpan) : AstValue
public data class AstString(
    val value: String,
    val raw: String,
    val parts: List<AstStringPart>,
    override val span: SourceSpan,
) : AstValue
public data class AstArray(val items: List<AstArrayItem>, override val span: SourceSpan) : AstValue
public data class AstSubstitution(val path: AstPath, override val span: SourceSpan) : AstValue

public data class AstObjectValue(val value: AstObject) : AstValue {
    override val span: SourceSpan = value.span
}

public sealed interface AstArrayItem : AstNode

public data class AstArrayValueItem(
    val value: AstValue,
    override val span: SourceSpan,
) : AstArrayItem

public data class AstArraySpread(
    val substitution: AstSubstitution,
    override val span: SourceSpan,
) : AstArrayItem

public sealed interface AstStringPart

public data class AstStringLiteralPart(val value: String) : AstStringPart

public data class AstStringInterpolationPart(
    val path: AstPath,
    val span: SourceSpan,
) : AstStringPart
