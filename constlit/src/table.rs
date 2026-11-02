//! Name and number tables shared by [`Signal`](crate::unix::Signal) and
//! [`Errno`](crate::linux::Errno), searched in `const fn`s.

/// `(name, number)` pairs; a number's first name is its canonical one.
pub(crate) type Table = [(&'static str, i32)];

pub(crate) const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The number of `name` (with its `SIG` or `E` prefix) in `table`.
pub(crate) const fn number_of(table: &Table, name: &str) -> Option<i32> {
    let mut i = 0;
    while i < table.len() {
        if str_eq(table[i].0, name) {
            return Some(table[i].1);
        }
        i += 1;
    }
    None
}

/// The canonical name of `number` in `table`: the first entry with that number.
pub(crate) const fn name_of(table: &Table, number: i32) -> Option<&'static str> {
    let mut i = 0;
    while i < table.len() {
        if table[i].1 == number {
            return Some(table[i].0);
        }
        i += 1;
    }
    None
}
