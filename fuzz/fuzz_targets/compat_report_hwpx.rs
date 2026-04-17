#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(core) = rhwp::document_core::DocumentCore::from_bytes(data) {
        let _ = core.compatibility_report_data();
        let _ = core.font_substitution_report_data();
    }
});
