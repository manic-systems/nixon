#![no_main]

use libfuzzer_sys::fuzz_target;
use nixon::{ElementId, parse_bytes};

fuzz_target!(|input: &[u8]| {
    let Ok(document) = parse_bytes(input) else {
        return;
    };
    assert_eq!(document.root().text().as_bytes(), input);

    for index in 0..document.element_count() {
        let element = document
            .element(ElementId::new(index as u32))
            .expect("element index is within the reported count");
        let range = element.range();
        assert!(range.start() <= range.end());
        assert!(range.end().as_usize() <= input.len());
        if index == 0 {
            assert!(element.parent().is_none());
        } else {
            assert!(element.parent().is_some());
        }
    }
});
