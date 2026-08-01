#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = nvide_ipc::read_frame(data, nvide_ipc::MAX_PAYLOAD);
});
