#![no_std]
//! minimize the indentation of a sequence of lines of text.
//!
//! eases processing of `indent_style = space` files by shrinking the indentation
//! down so that it is monotonically increasing. does not assume an `indent_size`
//! has been imposed to align everything to regular tab stops.
//!
//! an indentation of N spaces is unnecessary if no line uses exactly N spaces.
//! in that case all lines with more than N can be reduced by 1 without affecting
//! the logical indentation levels at all. after eliminating all such unnecessary
//! indentations the number of spaces remaining is equal to the logical level.

/// each line is divided into margin+content, both possibly empty.
/// designed for use in no_std zero-copy environments, see test code for examples.
/// everything is `pub` so the owner of the lines can freely alter their data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Line<'a> {
    /// must contain only spaces for [Line::minimize] to be used.
    pub margin: &'a str,
    /// arbitrary chars following the margin.
    pub content: &'a str,
}
impl<'a> Line<'a> {
    /// construct a Line using `value.trim_start_matches(' ')` to find margin.
    pub fn from(value: &'a str) -> Self {
        let content = value.trim_start_matches(' ');
        let trimmed = value.len() - content.len();
        let margin = &value[0..trimmed];
        Line { margin, content }
    }
    /// total length
    pub fn len(&self) -> usize {
        self.margin.len() + self.content.len()
    }
    /// the margin size
    pub fn indentation(&self) -> usize {
        self.margin.len() // assumes 7-bit
    }
    /// shrink margins down as much as possible
    /// panics if any margin isn't just `' '` chars.
    pub fn minimize(lines: &mut [Line]) {
        assert!(
            !lines
                .iter()
                .any(|it| !it.margin.trim_matches(' ').is_empty())
        );
        let mut level = 0usize;
        let mut found = lines
            .iter()
            .filter(|it| it.len() != 0)
            .map(|it| it.indentation())
            .min();
        while let Some(shrink) = found {
            for line in lines.iter_mut() {
                let indent = line.indentation();
                if indent > level {
                    line.margin = &line.margin[shrink..];
                }
            }
            level += 1;
            found = lines
                .iter()
                .filter(|it| it.indentation() > level)
                .map(|it| it.indentation() - level)
                .min();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::rust_2021::test;

    #[test]
    #[should_panic]
    fn bad_char() {
        let mut lines = [Line {
            margin: "\t",
            content: "ignored",
        }];
        Line::minimize(&mut lines);
    }

    const STACK_CAPACITY: usize = 20;
    fn check(expected: &[usize], literal: &str) {
        let mut array = [Line::default(); STACK_CAPACITY + 2];
        let mut index = 0usize;
        for line in literal.lines() {
            assert!(index < array.len(), "stack array isn't big enough");
            array[index] = Line::from(line);
            index += 1;
        }
        assert!(index > 1, "zero lines, nothing to check");
        assert!(array[0].len() == 0, "opening line must be empty");
        assert!(
            array[index - 1].content.is_empty(),
            "closing line must be blank"
        );
        let lines = &mut array[1..index - 1];
        Line::minimize(lines);
        assert_eq!(
            expected.len(),
            lines.len(),
            "expecting {} but only have {}",
            expected.len(),
            lines.len()
        );
        for ((index, check), line) in expected.iter().enumerate().zip(lines) {
            let num = index + 1;
            assert_eq!(
                *check,
                line.indentation(),
                "line {num} (expected == actual)"
            );
        }
    }

    #[test]
    #[should_panic]
    fn too_many() {
        let expected = [0; STACK_CAPACITY + 1];
        check(&expected, &"\n".repeat(expected.len() + 2));
    }
    #[test]
    fn at_limit() {
        let expected = [0; STACK_CAPACITY];
        check(&expected, &"\n".repeat(expected.len() + 2));
    }

    #[test]
    #[should_panic]
    fn too_few() {
        check(&[], r#""#);
    }
    #[test]
    fn zero_lines() {
        check(
            &[],
            r#"
            "#,
        )
    }

    #[test]
    fn one_line() {
        fn okay(it: &str) {
            // same outcome expected for all
            check(&[0], it);
        }
        okay(
            r#"
        rare
            "#,
        );
        okay(
            r#"
            unusual
            "#,
        );
        okay(
            r#"
                typical
            "#,
        );
        okay(
            r#"
                  excess
            "#,
        );
    }

    #[test]
    fn two_lines_same() {
        fn check(it: &str) {
            // same outcome expected for all
            crate::tests::check(&[0, 0], it);
        }
        check(
            r#"
        rare
        rare
            "#,
        );
        check(
            r#"
            unusual
            unusual
            "#,
        );
        check(
            r#"
                typical
                typical
            "#,
        );
        check(
            r#"
                  excess
                  excess
            "#,
        );
    }

    #[test]
    fn two_lines_2nd_more() {
        fn check(it: &str) {
            // same outcome expected for all
            crate::tests::check(&[0, 1], it);
        }
        check(
            r#"
        rare
                  excess
            "#,
        );
        check(
            r#"
            unusual
                  excess
            "#,
        );
        check(
            r#"
                typical
                  excess
            "#,
        );
    }

    #[test]
    fn two_lines_1st_more() {
        fn check(it: &str) {
            // same outcome expected for all
            crate::tests::check(&[1, 0], it);
        }
        check(
            r#"
                  excess
        rare
            "#,
        );
        check(
            r#"
                  excess
            unusual
            "#,
        );
        check(
            r#"
                  excess
                typical
            "#,
        );
    }

    #[test]
    fn four_lines() {
        check(
            &[3, 0, 2, 1],
            r#"
                  excess
        rare
                typical
            unusual
            "#,
        );
        check(
            &[2, 1, 3, 0],
            r#"
                typical
            unusual
                  excess
        rare
            "#,
        );
    }

    #[test]
    fn normal_case() {
        check(
            &[0, 0, 1, 2, 2, 1, 2, 0],
            r#"
                the first line usually establishes baseline indentation.
                second line here stays at root level, but has nesting:
                  + item 1
                    + item 1.1
                    + item 1.2
                  + item 2
                    + item 2.1
                back to root level to wrap things up.
            "#,
        );
    }

    #[test]
    fn unfortunate() {
        // wrote a really naive version of the algorithm in a proc_macro crate first.
        // having there was awkward, and figured it could be useful in other contexts.
        // so cut-n-paste to this crate, iron out wrinkles, then wrote the tests above.
        // focused all that time on the particulars of the code, not so much on how it
        // reacts to the bigger universe of inputs it might be used on....
        check(
            &[0, 1, 1, 0, 2, 2, 2],
            r#"
                *
                    *
                    *
                *
                     * oops wanted level 1
                     * but typo on first line was
                     * propagated by the editor
            "#,
        );
        // the way vscode works makes this scenario worth considering. it feels like
        // it would be preferable to recognize this and correct it, but not sure there
        // is enough information provided to pull that off. the simplicity of the
        // current algorithm is great, probably better to keep it that way.
    }

    #[test]
    fn empty_lines() {
        check(
            &[0, 0],
            r#"
                typical

            "#,
        );
        check(
            &[0, 0],
            r#"

                typical
            "#,
        );
        check(
            &[0, 0, 0],
            r#"
                typical


            "#,
        );
        check(
            &[0, 0, 0],
            r#"

                typical

            "#,
        );
        check(
            &[0, 0, 0],
            r#"


                typical
            "#,
        );
    }
}
