#!/usr/bin/env bash
set -euo pipefail

target="${1:-parse}"
seconds="${2:-30}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
corpus="${root}/fuzz/corpus/${target}"
findings="${root}/fuzz/findings/${target}"
bin="${root}/fuzz/bin/${target}"
artifacts="${root}/fuzz/artifacts"

case "${target}" in
  parse|format-source) ;;
  *)
    echo "unknown fuzz target: ${target}" >&2
    exit 2
    ;;
esac

rm -rf "${bin}"
mkdir -p "${corpus}" "${findings}" "${bin}" "${artifacts}"

archive_findings() {
  if [ -d "${findings}" ]; then
    tar -C "${root}/fuzz/findings" -czf "${artifacts}/${target}-findings.tar.gz" "${target}" 2>/dev/null || true
  fi
}
trap archive_findings EXIT
find "${root}/../tests/conformance" -name '*.scon' -type f -print0 |
  while IFS= read -r -d '' file; do
    cp "${file}" "${corpus}/$(basename "$(dirname "${file}")")-$(basename "${file}")"
  done
printf '' > "${corpus}/empty.scon"
printf '# comment\n' > "${corpus}/comment.scon"

dotnet publish "${root}/fuzz/Scon.Fuzz/Scon.Fuzz.csproj" -c Release -o "${bin}"
sharpfuzz "${bin}/Scon.Core.dll"

export AFL_SKIP_CPUFREQ=1
export AFL_SKIP_BIN_CHECK=1
export AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1
afl-fuzz -V "${seconds}" -i "${corpus}" -o "${findings}" -- dotnet "${bin}/Scon.Fuzz.dll" "${target}"
