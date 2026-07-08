use regex::Regex;
use std::sync::OnceLock;

/// Compiled, shared ANSI escape matcher. Ports the `ESC_SEQ` regex from
/// `src/lib/utils/ansi.ts` so Rust search targets match the JS-stripped text.
///
/// Alternation order mirrors the TS source:
///   1. SGR            ESC [ <params> m
///   2. other CSI      ESC [ <params> <final letter>
///   3. OSC            ESC ] … (BEL | ST=ESC \)
///   4. charset        ESC ( ) * + . / <char>
///   5. misc           ESC <char>   (also swallows a trailing lone ESC)
pub(crate) fn ansi_regex() -> &'static Regex {
    static ANSI: OnceLock<Regex> = OnceLock::new();
    ANSI.get_or_init(|| {
        Regex::new(concat!(
            r"\x1b\[[0-9;]*m",                 // SGR
            r"|\x1b\[[0-9;?]*[A-Za-z]",        // other CSI
            r"|\x1b\](?:[^\x07]|\x1b[^\\])*(?:\x07|\x1b\\)", // OSC
            r"|\x1b[()*+./].",                 // charset
            r"|\x1b.",                         // misc / lone ESC
        ))
        .expect("ansi escape regex must compile")
    })
}

/// Remove ANSI escape sequences, matching `stripAnsi` in `src/lib/utils/ansi.ts`.
pub fn strip_ansi(input: &str) -> String {
    ansi_regex().replace_all(input, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_sgr_color_codes() {
        assert_eq!(strip_ansi("\x1b[32mhello\x1b[39m"), "hello");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(strip_ansi("listening on 3000"), "listening on 3000");
    }

    #[test]
    fn removes_csi_cursor_codes() {
        assert_eq!(strip_ansi("a\x1b[2Kb"), "ab");
    }

    #[test]
    fn removes_osc_terminated_by_bel() {
        assert_eq!(strip_ansi("\x1b]0;title\x07body"), "body");
    }

    #[test]
    fn removes_osc_terminated_by_st() {
        assert_eq!(strip_ansi("\x1b]0;title\x1b\\body"), "body");
    }
}
