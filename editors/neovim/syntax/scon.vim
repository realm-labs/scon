if exists("b:current_syntax")
  finish
endif

syntax case match

syntax match sconComment "#.*$"
syntax match sconComment "//.*$"

syntax region sconString start=+"+ skip=+\\\\\|\\"+ end=+"+ contains=sconEscape,sconInterpolation
syntax match sconEscape +\\\(["\\/bfnrt$]\|u[0-9A-Fa-f]\{4}\)+ contained
syntax match sconInvalidEscape +\\.+ contained

syntax match sconInterpolation "\${[^}]*}" contained contains=sconInterpolationDelimiter,sconPath
syntax match sconInterpolation "\${[^}]*}" contains=sconInterpolationDelimiter,sconPath
syntax match sconInterpolationDelimiter "\${\|}" contained

syntax match sconNumber "\<-\=\(0\|[1-9][0-9]*\)\(\.[0-9]\+\)\=\([eE][+-]\=[0-9]\+\)\=\>"
syntax keyword sconBoolean true false
syntax keyword sconNull null
syntax keyword sconInclude include

syntax match sconSpread "\.\.\."
syntax match sconOperator "="
syntax match sconPath "\<[A-Za-z_][A-Za-z0-9_-]*\>" contained
syntax match sconKey "\<[A-Za-z_][A-Za-z0-9_-]*\>\ze\s*\(\.\|=\|{\)"
syntax match sconDelimiter "[,.\[\]{}]"

highlight default link sconComment Comment
highlight default link sconString String
highlight default link sconEscape SpecialChar
highlight default link sconInvalidEscape Error
highlight default link sconInterpolation Identifier
highlight default link sconInterpolationDelimiter Delimiter
highlight default link sconNumber Number
highlight default link sconBoolean Boolean
highlight default link sconNull Constant
highlight default link sconInclude Keyword
highlight default link sconSpread Operator
highlight default link sconOperator Operator
highlight default link sconPath Identifier
highlight default link sconKey Identifier
highlight default link sconDelimiter Delimiter

let b:current_syntax = "scon"
