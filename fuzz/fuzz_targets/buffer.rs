#![no_main]

use libfuzzer_sys::fuzz_target;
use nvide_buffer::{Buffer, CursorSnapshot, Edit, EditBatch, RopeBuffer};

fuzz_target!(|data: &[u8]| {
    let original = String::from_utf8_lossy(data);
    let mut buffer = RopeBuffer::new(&original);
    let before = buffer.text();
    let at = data.first().copied().unwrap_or(0) as usize % (buffer.len_chars() + 1);
    let insertion = String::from_utf8_lossy(data.get(1..).unwrap_or_default()).into_owned();
    let after = at + insertion.chars().count();
    let applied = buffer.apply_batch(EditBatch {
        edits: vec![Edit::Insert {
            at,
            text: insertion,
        }],
        before: CursorSnapshot {
            anchor: at,
            head: at,
        },
        after: CursorSnapshot {
            anchor: after,
            head: after,
        },
    });
    if applied.is_ok() {
        assert!(buffer.undo().is_ok());
        assert_eq!(buffer.text(), before);
    }
});
