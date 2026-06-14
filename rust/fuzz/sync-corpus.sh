#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixtures="${root}/tests/conformance"
fuzz_root="${root}/rust/fuzz"

for target in parse_str format_source; do
  corpus="${fuzz_root}/corpus/${target}"
  mkdir -p "${corpus}"
  find "${fixtures}" -name '*.scon' -type f -print0 |
    while IFS= read -r -d '' file; do
      cp "${file}" "${corpus}/$(basename "$(dirname "${file}")")-$(basename "${file}")"
    done
  printf '' > "${corpus}/empty.scon"
  printf '# comment\n' > "${corpus}/comment.scon"
done
