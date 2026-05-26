use matchkit::Match;
use proptest::prelude::*;

fn mk(pattern_id: u32, start: u32, end: u32) -> Match {
    Match::new(pattern_id, start, end)
}

fn normalized(pattern_id: u32, a: u32, b: u32) -> Match {
    let start = a.min(b);
    let end = a.max(b);
    mk(pattern_id, start, end)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn smoke(x in 0..10i32) {
        prop_assert!(x >= 0);
    }

    #[test]
    fn smoke2(a in any::<u32>(), b in any::<u32>()) {
        prop_assert!(a <= u32::MAX);
        let _ = b;
    }

    #[test]
    fn p11_match_new_fields(
        pattern_id in any::<u32>(),
        start in any::<u32>(),
        end in any::<u32>(),
    ) {
        let m = Match::new(pattern_id, start, end);
        prop_assert_eq!(m.pattern_id, pattern_id);
    }

    #[test]
    fn p24_extend_empty_noop() {
        let mut set = MatchSet::new();
        set.extend(std::iter::empty::<Match>());
        prop_assert!(set.is_empty());
    }
}
