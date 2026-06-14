#!/usr/bin/env bash
set -euo pipefail

target="${1:-parse}"
seconds="${2:-30}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "${target}" in
  parse)
    source_file="${root}/Fuzz/parse_string.swift"
    output="${root}/.build/fuzz/parse"
    ;;
  format-source)
    source_file="${root}/Fuzz/format_source.swift"
    output="${root}/.build/fuzz/format-source"
    ;;
  *)
    echo "unknown fuzz target: ${target}" >&2
    exit 2
    ;;
esac

cd "${root}"
swift build

mkdir -p "$(dirname "${output}")" "${root}/fuzz/artifacts" "${root}/fuzz/corpus/${target}"

if swiftc -parse-as-library -sanitize=fuzzer,address Sources/SconCore/*.swift "${source_file}" -o "${output}" 2>"${root}/fuzz/artifacts/swiftc-${target}.log"; then
  "${output}" "-max_total_time=${seconds}" "-artifact_prefix=${root}/fuzz/artifacts/"
  exit 0
fi

echo "swiftc -sanitize=fuzzer is unavailable; falling back to seed replay" >&2
replay="${root}/.build/fuzz/replay"
swiftc Sources/SconCore/*.swift Fuzz/replay.swift -o "${replay}"

find "${root}/../tests/conformance" -name '*.scon' -type f -print0 |
  while IFS= read -r -d '' file; do
    cp "${file}" "${root}/fuzz/corpus/${target}/$(basename "$(dirname "${file}")")-$(basename "${file}")"
  done
printf '' > "${root}/fuzz/corpus/${target}/empty.scon"
printf '# comment\n' > "${root}/fuzz/corpus/${target}/comment.scon"

find "${root}/fuzz/corpus/${target}" -type f -print0 |
  while IFS= read -r -d '' file; do
    "${replay}" "${target}" < "${file}"
  done
