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
}
