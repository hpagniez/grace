//! `consoleFind.test.ts`, test for test. Offsets are bytes (see the module).
use super::*;

fn m(line: usize, start: usize, end: usize) -> Match {
    Match { line, start, end }
}

mod find_matches {
    use super::*;

    #[test]
    fn counts_every_occurrence_case_aside_without_overlap() {
        assert_eq!(
            find_matches(["get user 42", "create USER", "aaaa"], "user"),
            [m(0, 4, 8), m(1, 7, 11)]
        );
        assert_eq!(find_matches(["aaaa"], "aa"), [m(0, 0, 2), m(0, 2, 4)]);
    }

    #[test]
    fn takes_the_text_as_plain_text() {
        assert_eq!(find_matches(["a.b(c", "axb(c"], "a.b("), [m(0, 0, 4)]);
        assert_eq!(find_matches(["1*2\\3"], "*2\\"), [m(0, 1, 4)]);
    }

    #[test]
    fn keeps_offsets_on_the_line_as_written_when_case_folding_changes_its_length() {
        // "İ" is two bytes; lower-cased it is three
        assert_eq!(find_matches(["İstanbul user"], "user"), [m(0, 10, 14)]);
        assert_eq!(find_matches(["İstanbul user"], "İSTANBUL"), [m(0, 0, 9)]);
    }

    #[test]
    fn keeps_the_spaces_of_the_text() {
        assert_eq!(find_matches(["a ERROR b", "aERRORb"], " ERROR "), [m(0, 1, 8)]);
    }

    #[test]
    fn finds_nothing_for_no_text() {
        assert_eq!(find_matches(["a"], ""), []);
    }
}

mod error_lines {
    use super::*;

    #[test]
    fn takes_error_lines_and_failed_tests_not_warn_lines() {
        // Karate 2.1.4's own lines, and its end-of-run summary
        let lines = [
            "09:46:08.476 [virtual-36] INFO  karate.scenario - hello",
            "09:46:08.502 [virtual-40] WARN  karate.scenario - careful",
            "09:46:08.502 [virtual-46] ERROR karate.scenario - [ERROR] custom boom",
            "failed features:",
            "  C:\\work\\a.feature",
            "    - fails match",
            "      C:\\work\\a.feature:7",
            "      ==========",
            "    - line 2",
            "============================================================",
            "    - after the block",
        ];
        assert_eq!(error_lines(lines), [2, 5, 8]);
    }

    #[test]
    fn starts_the_block_only_at_the_exact_line() {
        let lines = [
            "12:00 [main] INFO  karate.scenario - failed features:",
            "    - not a test",
        ];
        assert_eq!(error_lines(lines), Vec::<usize>::new());
    }
}

mod stepping {
    use super::*;

    #[test]
    fn starts_from_the_top_of_the_view_wrapping() {
        assert_eq!(start_index(&[3, 7, 9], 5, true), Some(1));
        assert_eq!(start_index(&[3, 7, 9], 5, false), Some(0));
        assert_eq!(start_index(&[3, 7, 9], 10, true), Some(0));
        assert_eq!(start_index(&[3, 7, 9], 0, false), Some(2));
        assert_eq!(start_index(&[], 0, true), None);
    }

    #[test]
    fn steps_round() {
        assert_eq!(step_index(3, 2, true), 0);
        assert_eq!(step_index(3, 0, false), 2);
    }
}
