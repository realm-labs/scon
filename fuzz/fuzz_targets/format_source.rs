#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(formatted) = scon::format_source(source, scon::FormatOptions::default()) else {
        return;
    };

    scon::parse_source(&formatted, scon::ParseOptions::default())
        .expect("formatted SCON should parse");

    if let (Ok(original), Ok(round_trip)) = (scon::parse_str(source), scon::parse_str(&formatted)) {
        assert_eq!(original, round_trip, "formatting changed resolved value");
    }
});
