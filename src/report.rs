/// Human readable byte count using binary units.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// `A is larger by 1.2 GiB (A 3.0 GiB, B 1.8 GiB)` from the two sides.
pub fn size_difference(a: u64, b: u64) -> String {
    let larger = if a > b { "A" } else { "B" };
    let delta = a.abs_diff(b);
    format!(
        "{larger} is larger by {} (A {}, B {})",
        size(delta),
        size(a),
        size(b)
    )
}

/// `file` or `files` for a count.
pub fn plural(value: u64) -> &'static str {
    if value == 1 { "file" } else { "files" }
}

/// Group digits in threes, e.g. `12483` becomes `12,483`.
pub fn count(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_use_binary_units() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1024), "1.0 KiB");
        assert_eq!(size(1536), "1.5 KiB");
        assert_eq!(size(100 << 20), "100.0 MiB");
        assert_eq!(size(100u64 << 30), "100.0 GiB");
        assert_eq!(size(3u64 << 40), "3.0 TiB");
    }

    #[test]
    fn size_difference_names_the_larger_side() {
        assert_eq!(
            size_difference(3 << 30, 1 << 30),
            "A is larger by 2.0 GiB (A 3.0 GiB, B 1.0 GiB)"
        );
        assert_eq!(
            size_difference(100, 1024),
            "B is larger by 924 B (A 100 B, B 1.0 KiB)"
        );
    }

    #[test]
    fn plurals_follow_the_count() {
        assert_eq!(plural(0), "files");
        assert_eq!(plural(1), "file");
        assert_eq!(plural(2), "files");
    }

    #[test]
    fn counts_group_digits_in_threes() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(12483), "12,483");
        assert_eq!(count(1234567), "1,234,567");
    }
}
