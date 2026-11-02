//! Random cron fields with their masks, shared by `constlit/tests/cron_next.rs`
//! (which checks the masks against croner) and `constlit-macros`' unit tests
//! (which check them against `parse_cron`).

pub fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// One random field and its mask: `*` or 1-3 items from `min..=max`.
/// `day_field` leaves out `*`-prefixed items other than a bare `*`, where croner's
/// day rule differs; `matches_vixie` pins those.
pub fn field(state: &mut u64, min: u64, max: u64, day_field: bool) -> (String, u64) {
    let range =
        |a: u64, b: u64, step: u64| (a..=b).step_by(step as usize).fold(0, |m, v| m | 1 << v);
    if splitmix(state).is_multiple_of(4) {
        return ("*".into(), range(min, max, 1));
    }
    let span = max - min + 1;
    let mut r = |n: u64| splitmix(state) % n;
    let mut mask = 0;
    let items: Vec<String> = (0..1 + r(3))
        .map(|_| {
            let a = min + r(span);
            let b = a + r(max - a + 1);
            let step = 1 + r(span.min(15));
            let (text, bits) = match (r(5), day_field) {
                (0, _) => (a.to_string(), range(a, a, 1)),
                (1, _) => (format!("{a}-{b}"), range(a, b, 1)),
                (2, _) => (format!("{a}-{b}/{step}"), range(a, b, step)),
                (3, false) => (format!("*/{step}"), range(min, max, step)),
                // croner rejects `a/n`; `a-max/n` means the same and `cron_values` pins `a/n`.
                (_, false) => (format!("{a}-{max}/{step}"), range(a, max, step)),
                (_, true) => (format!("{a},{b}"), range(a, a, 1) | range(b, b, 1)),
            };
            mask |= bits;
            text
        })
        .collect();
    (items.join(","), mask)
}
