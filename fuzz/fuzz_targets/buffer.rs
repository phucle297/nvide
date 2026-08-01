#![no_main]

use libfuzzer_sys::fuzz_target;
use nvide_buffer::{Buffer, CursorSnapshot, Edit, EditBatch, RopeBuffer};

fuzz_target!(|data: &[u8]| {
    let mut buffer = RopeBuffer::new("λ\r\nseed\n");
    for byte in data.iter().copied().take(64) {
        let len = buffer.len_chars();
        let at = byte as usize % (len + 1);
        let text = match (byte / 4) % 4 {
            0 => "λ",
            1 => "\r\n",
            2 => "x",
            _ => "🦀\n",
        };
        let edits = match byte % 4 {
            0 => vec![Edit::Insert {
                at,
                text: text.to_owned(),
            }],
            1 if at < len => vec![Edit::Delete { range: at..at + 1 }],
            2 if at < len => vec![
                Edit::Delete { range: at..at + 1 },
                Edit::Insert {
                    at,
                    text: text.to_owned(),
                },
            ],
            3 => vec![Edit::Delete {
                range: at..len + 1,
            }],
            _ => vec![Edit::Insert {
                at,
                text: text.to_owned(),
            }],
        };
        let final_len = edits.iter().fold(len, |current, edit| match edit {
            Edit::Insert { text, .. } => current + text.chars().count(),
            Edit::Delete { range } if range.end <= current => current - (range.end - range.start),
            Edit::Delete { .. } => current,
        });
        let before = (buffer.text(), buffer.version());
        let applied = buffer.apply_batch(EditBatch {
            edits,
            before: CursorSnapshot {
                anchor: at,
                head: at,
            },
            after: CursorSnapshot {
                anchor: at.min(final_len),
                head: at.min(final_len),
            },
        });
        match applied {
            Ok(outcome) => {
                let after = buffer.text();
                assert_eq!(outcome.version, before.1 + 1);
                assert!(buffer.undo().is_ok());
                assert_eq!(buffer.text(), before.0);
                assert!(buffer.redo(Some(outcome.node)).is_ok());
                assert_eq!(buffer.text(), after);
            }
            Err(_) => assert_eq!(before, (buffer.text(), buffer.version())),
        }
    }
});
