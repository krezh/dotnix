use std::cmp::Ordering;

pub(super) fn compare_versions(left: &str, right: &str) -> Ordering {
    let mut left = Components(left);
    let mut right = Components(right);
    loop {
        let left = left.next();
        let right = right.next();
        if left.is_none() && right.is_none() {
            return Ordering::Equal;
        }
        let ordering = compare_components(left.unwrap_or_default(), right.unwrap_or_default());
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
}

struct Components<'a>(&'a str);

impl<'a> Iterator for Components<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.0 = self.0.trim_start_matches(['.', '-']);
        let first = self.0.bytes().next()?;
        let numeric = first.is_ascii_digit();
        let end = self
            .0
            .bytes()
            .position(|byte| matches!(byte, b'.' | b'-') || byte.is_ascii_digit() != numeric)
            .unwrap_or(self.0.len());
        let (component, rest) = self.0.split_at(end);
        self.0 = rest;
        Some(component)
    }
}

fn compare_components(left: &str, right: &str) -> Ordering {
    // Nix treats a component as numeric only when it fits in a signed int.
    let left_number = left.parse::<i32>().ok();
    let right_number = right.parse::<i32>().ok();
    if let (Some(left), Some(right)) = (left_number, right_number) {
        left.cmp(&right)
    } else if (left.is_empty() && right_number.is_some()) || (left == "pre" && right != "pre") {
        Ordering::Less
    } else if right == "pre" && left != "pre" {
        Ordering::Greater
    } else if right_number.is_some() {
        Ordering::Less
    } else if left_number.is_some() {
        Ordering::Greater
    } else {
        left.cmp(right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_nix_component_ordering() {
        for (left, right) in [
            ("1.0pre1", "1.0"),
            ("1.2.9", "1.2.10"),
            ("1.0", "1.0.1"),
            ("1.0", "1.0.0"),
            ("1.pre", "1"),
            ("1", "1a"),
            ("2147483648", "3"),
            ("2.4a", "2.4b"),
            ("2.3a", "2.3.1"),
            ("1.0pre", "1.0a"),
            ("1.0pre", "1.0.0"),
            ("1.0", "1.0a"),
            ("", "0"),
            ("pre", ""),
            ("pre", "a"),
            ("a", "1"),
        ] {
            assert_eq!(
                compare_versions(left, right),
                Ordering::Less,
                "{left} < {right}"
            );
            assert_eq!(
                compare_versions(right, left),
                Ordering::Greater,
                "{right} > {left}"
            );
        }
    }

    #[test]
    fn ignores_separators_and_numeric_leading_zeros() {
        for (left, right) in [
            ("", ""),
            ("", ".---.."),
            ("1.0", "-1--0.."),
            ("01.0002", "1.2"),
            ("01", "1"),
            ("000000000000000000000000000000000000001", "1"),
            ("pre", "pre"),
            ("1pre01", "1-pre-1"),
        ] {
            assert_eq!(
                compare_versions(left, right),
                Ordering::Equal,
                "{left} == {right}"
            );
        }
    }

    #[test]
    fn treats_overflowing_numbers_as_text() {
        for (left, right) in [
            ("2147483648", "2147483647"),
            ("99999999999999999999999999999999", "1"),
            ("2147483648", "9999999999"),
            ("1.2147483648", "1.2"),
            ("pre", "2147483648"),
            ("", "2147483648"),
        ] {
            assert_eq!(
                compare_versions(left, right),
                Ordering::Less,
                "{left} < {right}"
            );
            assert_eq!(
                compare_versions(right, left),
                Ordering::Greater,
                "{right} > {left}"
            );
        }
        assert_eq!(compare_versions("02147483647", "2147483647"), Ordering::Equal);
        assert_eq!(compare_versions("02147483648", "2147483648"), Ordering::Less);
    }

    #[test]
    fn separates_signs_from_numeric_components() {
        assert_eq!(compare_versions("+1", "1"), Ordering::Less);
        assert_eq!(compare_versions("-1", "1"), Ordering::Equal);
        assert_eq!(compare_versions("1+2", "1.2"), Ordering::Less);
    }
}
