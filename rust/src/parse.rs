use super::*;

#[allow(unused)]
struct Input<'a> {
    src: &'a str,
    next: usize,
    indent: usize,
}
#[allow(unused)]
impl<'a> Input<'a> {
    #[allow(unused)]
    fn encoded(&mut self, from: &'a str, start: usize) -> Encoded<'a> {
        let bytes = &from.as_bytes()[start..];
        let mut newlines = 0usize;
        let indent = self.indent + 1;
        let mut cursor = 0usize;
        'outer: while cursor < bytes.len() {
            if bytes[cursor] != b'\n' {
                cursor += 1;
                continue;
            }
            if cursor + indent >= bytes.len() {
                break;
            }
            for offset in 0..indent {
                if bytes[cursor + 1 + offset] != b'\t' {
                    break 'outer;
                }
            }
            cursor += 1 + indent;
            newlines += 1;
        }
        Encoded {
            utf8: &from[..cursor],
            dedent: if newlines == 0 { usize::MAX } else { indent },
        }
    }
}
