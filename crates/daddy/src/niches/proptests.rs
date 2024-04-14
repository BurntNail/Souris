use crate::{niches::integer::Integer, utilities::cursor::Cursor};
use alloc::string::ToString;
use core::str::FromStr;
use proptest::prelude::*;

proptest! {
    #[test]
    fn doesnt_crash (s in "\\PC*") {
        let _ = Integer::from_str(&s);
    }

    #[test]
    fn parse_valids (s in "-?[0-9]{1,19}") {
        let _ = Integer::from_str(&s).unwrap();
    }

    #[test]
    fn back_to_original (i in -1_000_000_i64..1_000_000_i64) {
        let s = i.to_string();
        let parsed = Integer::from_str(&s).unwrap();
        let sered = parsed.ser();
        let got_back = Integer::deser(&mut Cursor::new(&sered)).unwrap();
        assert_eq!(parsed, got_back);
    }
}
