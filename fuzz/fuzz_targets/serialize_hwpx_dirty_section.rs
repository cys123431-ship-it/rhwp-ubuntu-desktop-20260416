#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(core) = rhwp::document_core::DocumentCore::from_bytes(data) {
        if core.source_format() == rhwp::document_core::DocumentSourceFormat::Hwpx {
            let _ = core.export_hwpx_native();
        }
    }
});
