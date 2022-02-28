use std::collections::{HashMap, HashSet};

use askama::Template;

use crate::{
    context::all_matched_lines_filled,
    hunks::{matched_lines_for_hunk, Hunk},
    lines::{codepoint_len, LineNumber},
    positions::SingleLineSpan,
    side_by_side::{lines_with_novel, split_on_newlines},
    syntax::{AtomKind, MatchKind, MatchedPos, TokenKind},
};

type StyledLine = Vec<(String, Vec<&'static str>)>;
type NumberedLine = (LineNumber, StyledLine);

#[derive(Template)]
#[template(path = "summary.html")]
struct SummaryTemplate {
    display_path: String,
    paired_lines: Vec<(Option<NumberedLine>, Option<NumberedLine>)>,
    lhs_lines_with_novel: HashSet<LineNumber>,
    rhs_lines_with_novel: HashSet<LineNumber>,
}

fn apply_line(
    line: &str,
    styles: &[(SingleLineSpan, Vec<&'static str>)],
) -> Vec<(String, Vec<&'static str>)> {
    let mut offset = 0;
    let mut res = vec![];

    for (span, classes) in styles {
        if offset < span.start_col {
            res.push((line[offset..span.start_col].to_owned(), vec![]));
        }

        res.push((
            line[span.start_col..span.end_col].to_owned(),
            classes.clone(),
        ));
        offset = span.end_col;
    }
    if offset < codepoint_len(line) {
        res.push((line[offset..].to_owned(), vec![]));
    }

    res
}

fn apply_styles(
    is_lhs: bool,
    mps: &[MatchedPos],
) -> HashMap<LineNumber, Vec<(SingleLineSpan, Vec<&'static str>)>> {
    let mut line_styles = HashMap::new();
    for mp in mps {
        let line_pos = mp.pos;
        let mut span_classes = vec![];
        match mp.kind {
            MatchKind::Novel { .. }
            | MatchKind::NovelWord { .. } => {
                span_classes.push(if is_lhs { "novel-lhs" } else { "novel-rhs" });
            }
            MatchKind::UnchangedToken { .. } => {}
            MatchKind::NovelLinePart { .. } => {
                // We don't want extra highlighting for a line: the
                // relevant part is highlighted as NovelWord.
            }
        }

        let highlight = match mp.kind {
            MatchKind::UnchangedToken { highlight, .. } => highlight,
            MatchKind::Novel { highlight } => highlight,
            MatchKind::NovelLinePart { highlight, .. } => highlight,
            MatchKind::NovelWord { highlight } => highlight,
        };

        match highlight {
            TokenKind::Atom(kind) => match kind {
                AtomKind::Normal => {}
                AtomKind::String => span_classes.push("pl-s"),
                AtomKind::Type => span_classes.push("pl-k"),
                AtomKind::Comment => span_classes.push("pl-c"),
                AtomKind::Keyword => span_classes.push("pl-k"),
            },
            _ => {}
        }

        let line_classes = line_styles.entry(line_pos.line).or_insert_with(Vec::new);
        line_classes.push((line_pos, span_classes));
    }

    line_styles
}

pub fn print(
    hunks: &[Hunk],
    display_path: &str,
    lhs_src: &str,
    rhs_src: &str,
    lhs_mps: &[MatchedPos],
    rhs_mps: &[MatchedPos],
) {
    let lhs_lines = split_on_newlines(lhs_src);
    let rhs_lines = split_on_newlines(rhs_src);
    let lhs_line_styles = apply_styles(true, lhs_mps);
    let rhs_line_styles = apply_styles(false, rhs_mps);
    let empty_styles = vec![];

    let (lhs_lines_with_novel, rhs_lines_with_novel) = lines_with_novel(lhs_mps, rhs_mps);

    let matched_lines = all_matched_lines_filled(lhs_mps, rhs_mps);

    let mut paired_lines: Vec<(Option<NumberedLine>, Option<NumberedLine>)> = vec![];
    for hunk in hunks {
        let aligned_lines = matched_lines_for_hunk(&matched_lines, hunk);
        for (lhs_num, rhs_num) in aligned_lines {
            let lhs = lhs_num.map(|ln| {
                (
                    ln,
                    apply_line(
                        lhs_lines[ln.0],
                        lhs_line_styles.get(&ln).unwrap_or(&empty_styles),
                    ),
                )
            });
            let rhs = rhs_num.map(|ln| {
                (
                    ln,
                    apply_line(
                        rhs_lines[ln.0],
                        rhs_line_styles.get(&ln).unwrap_or(&empty_styles),
                    ),
                )
            });
            paired_lines.push((lhs, rhs));
        }
    }

    let template = SummaryTemplate {
        display_path: display_path.into(),
        paired_lines,
        lhs_lines_with_novel,
        rhs_lines_with_novel,
    };
    println!("{}", template.render().unwrap());
}