module.exports = grammar({
  name: "scon",

  word: $ => $.identifier,

  extras: $ => [
    /[ \t\r\n]/,
    $.comment,
  ],

  rules: {
    document: $ => optional(choice(
      $.object,
      $.object_body,
    )),

    object: $ => seq("{", optional($.object_body), "}"),

    object_body: $ => repeat1(seq(
      choice(
        $.object_spread,
        $.include_directive,
        $.field,
      ),
      optional(","),
    )),

    include_directive: $ => seq(
      alias($.include_keyword, "include"),
      $.string,
    ),

    field: $ => seq(
      $.path,
      choice(
        seq("=", $.value),
        $.object,
      ),
    ),

    value: $ => choice(
      $.object,
      $.array,
      $.string,
      $.number,
      $.boolean,
      $.null,
      $.substitution,
    ),

    array: $ => seq(
      "[",
      optional(seq(
        $.array_item,
        repeat(seq(",", $.array_item)),
        optional(","),
      )),
      "]",
    ),

    array_item: $ => choice(
      $.array_spread,
      $.value,
    ),

    object_spread: $ => seq("...", $.substitution),

    array_spread: $ => seq("...", $.substitution),

    substitution: $ => seq("${", $.path, "}"),

    path: $ => seq(
      $.path_segment,
      repeat(seq(".", $.path_segment)),
    ),

    path_segment: $ => choice(
      $.identifier,
      $.string,
    ),

    boolean: _ => choice("true", "false"),

    null: _ => "null",

    include_keyword: _ => token(prec(1, "include")),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_-]*/,

    number: _ => token(/-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/),

    string: _ => token(seq(
      "\"",
      repeat(choice(
        /[^"\\\n\r]/,
        /\\["\\\/bfnrt$]/,
        /\\u[0-9A-Fa-f]{4}/,
      )),
      "\"",
    )),

    comment: _ => token(choice(
      seq("#", /[^\r\n]*/),
      seq("//", /[^\r\n]*/),
    )),
  },
});
