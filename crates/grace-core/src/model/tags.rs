//! `src/shared/tags.ts`: the tag filter. Tags are stored and compared without '@', case-sensitive.
use super::js;
use super::node::TestNode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// A project's tag filter. Electron Grace's `TagFilter` is this without `hide_non_matching`: functions that take one
/// take a `FilterState` and ignore the flag.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterState {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// Hide tests and tree nodes that do not match; dim them when false.
    pub hide_non_matching: bool,
}

/// No tag included or excluded, non-matching tests dimmed.
pub fn no_filter() -> FilterState {
    FilterState::default()
}

const MAX_TAGS: usize = 500;
const MAX_TAG_LENGTH: usize = 200;

pub fn filter_on(f: &FilterState) -> bool {
    !f.include.is_empty() || !f.exclude.is_empty()
}

/// Whether `-t` can select the tag. Karate splits `-t` on commas and turns it into JS without escaping (`'` and `(`
/// fail the run); a leading `@` or `~` would change the expression.
///
/// The TypeScript's regular expression `^[^\s,'(\\@~][^\s,'(\\]*$`, written as the two character sets it is: `\s` is
/// JavaScript's (`js::is_space`), not Rust's.
pub fn filterable(tag: &str) -> bool {
    let rest_ok = |c: char| !(js::is_space(c) || matches!(c, ',' | '\'' | '(' | '\\'));
    let mut chars = tag.chars();
    chars
        .next()
        .is_some_and(|first| rest_ok(first) && !matches!(first, '@' | '~'))
        && chars.all(rest_ok)
}

/// A test matches when it has any included tag (or none are included) and no excluded tag.
pub fn matches(tags: &[String], f: &FilterState) -> bool {
    (f.include.is_empty() || tags.iter().any(|t| f.include.contains(t))) && !tags.iter().any(|t| f.exclude.contains(t))
}

/// Scenario and row nodes at or under `n` that match, in tree order. Borrowed from the tree, where the TypeScript
/// returns the same node objects.
pub fn matching_leaves<'a>(n: &'a TestNode, f: &FilterState) -> Vec<&'a TestNode> {
    fn walk<'a>(n: &'a TestNode, f: &FilterState, out: &mut Vec<&'a TestNode>) {
        if n.is_leaf() {
            if matches(&n.tags, f) {
                out.push(n);
            }
        } else {
            n.children.iter().for_each(|c| walk(c, f, out));
        }
    }
    let mut out = Vec::new();
    walk(n, f, &mut out);
    out
}

/// Whether any test under `n` matches: `!matching_leaves(n, f).is_empty()`, stopping at the first one, without listing
/// them (#320: the toolbar asks at every frame).
pub fn any_matching_leaf(n: &TestNode, f: &FilterState) -> bool {
    if n.is_leaf() {
        matches(&n.tags, f)
    } else {
        n.children.iter().any(|c| any_matching_leaf(c, f))
    }
}

/// Sorted union of the filterable tags of every scenario and row. Sorted as JavaScript's `sort()` does, by UTF-16 unit.
pub fn project_tags(tree: &TestNode) -> Vec<String> {
    fn walk<'a>(n: &'a TestNode, tags: &mut HashSet<&'a str>) {
        if n.is_leaf() {
            tags.extend(n.tags.iter().map(String::as_str).filter(|t| filterable(t)));
        }
        n.children.iter().for_each(|c| walk(c, tags));
    }
    let mut tags = HashSet::new();
    walk(tree, &mut tags);
    let mut tags: Vec<String> = tags.into_iter().map(String::from).collect();
    js::sort_utf16(&mut tags);
    tags
}

/// Tags to show as chips: the project's, plus filtered ones that vanished until the user prunes them.
pub fn chip_tags(tree: Option<&TestNode>, f: &FilterState) -> Vec<String> {
    let mut tags: HashSet<String> = tree.map(project_tags).unwrap_or_default().into_iter().collect();
    tags.extend(f.include.iter().chain(&f.exclude).cloned());
    let mut tags: Vec<String> = tags.into_iter().collect();
    js::sort_utf16(&mut tags);
    tags
}

/// off -> include -> exclude -> off.
///
/// Takes the filter and gives it back, changed in place, where the TypeScript copies it.
pub fn cycle_tag(mut f: FilterState, tag: &str) -> FilterState {
    if f.include.iter().any(|t| t == tag) {
        f.include.retain(|t| t != tag);
        f.exclude.push(tag.to_string());
    } else if f.exclude.iter().any(|t| t == tag) {
        f.exclude.retain(|t| t != tag);
    } else {
        f.include.push(tag.to_string());
    }
    f
}

/// Drops the tags the project no longer has. Takes and gives back the filter, like `cycle_tag`.
pub fn prune_filter(mut f: FilterState, tags: &[String]) -> FilterState {
    f.include.retain(|t| tags.contains(t));
    f.exclude.retain(|t| tags.contains(t));
    f
}

/// A list of at most `MAX_TAGS` tags that `-t` can select, none over `MAX_TAG_LENGTH` UTF-16 units.
fn tag_list(x: Option<&Value>) -> Option<Vec<String>> {
    let list = x?.as_array().filter(|a| a.len() <= MAX_TAGS)?;
    list.iter()
        .map(|t| {
            t.as_str()
                .filter(|t| filterable(t) && js::utf16_len(t) <= MAX_TAG_LENGTH)
                .map(String::from)
        })
        .collect()
}

/// Validates untrusted input (the settings file, the window).
pub fn parse_filter_state(x: &Value) -> Option<FilterState> {
    let o = x.as_object()?;
    let include = tag_list(o.get("include"))?;
    let exclude = tag_list(o.get("exclude"))?;
    let hide_non_matching = o.get("hideNonMatching")?.as_bool()?;
    if include.iter().any(|t| exclude.contains(t)) {
        return None;
    }
    Some(FilterState {
        include,
        exclude,
        hide_non_matching,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::node::NodeKind;
    use serde_json::json;

    fn node(kind: NodeKind, key: &str, name: &str, tags: &[&str], children: Vec<TestNode>) -> TestNode {
        TestNode {
            key: key.into(),
            kind,
            name: name.into(),
            file: "tests/x.feature".into(),
            line: None,
            ref_id: None,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            example: None,
            children,
        }
    }

    fn scenario(n: u32, name: &str, tags: &[&str]) -> TestNode {
        node(
            NodeKind::Scenario,
            &format!("s:tests/x.feature#{n}"),
            name,
            tags,
            vec![],
        )
    }

    fn row(n: u32) -> TestNode {
        node(
            NodeKind::Row,
            &format!("s:tests/outline.feature#1.{n}"),
            &format!("add {n}"),
            &[],
            vec![],
        )
    }

    /// The clean sample project: 9 tests, tagged as in the tag filter spec.
    fn tree() -> TestNode {
        node(
            NodeKind::Folder,
            "d:",
            "clean",
            &[],
            vec![node(
                NodeKind::Folder,
                "d:tests",
                "tests",
                &[],
                vec![
                    node(
                        NodeKind::Feature,
                        "f:tests/basic.feature",
                        "basic",
                        &[],
                        vec![
                            scenario(1, "passing hello", &["smoke"]),
                            scenario(2, "failing match", &["wip"]),
                            scenario(3, "calls helper feature", &[]),
                            scenario(4, "smoke but wip", &["smoke", "wip"]),
                        ],
                    ),
                    node(
                        NodeKind::Feature,
                        "f:tests/outline.feature",
                        "outline",
                        &[],
                        vec![node(
                            NodeKind::Outline,
                            "o:tests/outline.feature#1",
                            "add",
                            &[],
                            vec![row(1), row(2), row(3)],
                        )],
                    ),
                    node(
                        NodeKind::Feature,
                        "f:tests/slow.feature",
                        "slow",
                        &[],
                        vec![
                            scenario(1, "slow loop", &["slow"]),
                            scenario(2, "smoke and slow", &["smoke", "slow"]),
                        ],
                    ),
                ],
            )],
        )
    }

    fn filter(include: &[&str], exclude: &[&str]) -> FilterState {
        FilterState {
            include: include.iter().map(|t| t.to_string()).collect(),
            exclude: exclude.iter().map(|t| t.to_string()).collect(),
            hide_non_matching: false,
        }
    }

    fn names(include: &[&str], exclude: &[&str]) -> Vec<String> {
        matching_leaves(&tree(), &filter(include, exclude))
            .into_iter()
            .map(|n| n.name.clone())
            .collect()
    }

    mod matching {
        use super::*;

        #[test]
        fn empty_filter_matches_everything() {
            assert_eq!(matching_leaves(&tree(), &no_filter()).len(), 9);
            assert!(!filter_on(&no_filter()));
        }

        #[test]
        fn include_smoke_exclude_wip() {
            assert_eq!(names(&["smoke"], &["wip"]), ["passing hello", "smoke and slow"]);
        }

        #[test]
        fn exclude_only_keeps_untagged_tests() {
            assert_eq!(names(&[], &["wip"]).len(), 7);
        }

        #[test]
        fn included_tags_combine_with_or() {
            assert_eq!(
                names(&["wip", "slow"], &[]),
                ["failing match", "smoke but wip", "slow loop", "smoke and slow"]
            );
        }

        #[test]
        fn matching_is_case_sensitive() {
            assert!(!matches(&["Smoke".to_string()], &filter(&["smoke"], &[])));
        }

        #[test]
        fn filter_on_is_true_with_an_include_or_an_exclude() {
            assert!(filter_on(&filter(&["a"], &[])));
            assert!(filter_on(&filter(&[], &["a"])));
        }

        #[test]
        fn any_matching_leaf_says_whether_matching_leaves_has_one() {
            for (include, exclude) in [
                (&[][..], &[][..]),
                (&["smoke"], &["wip"]),
                (&["nothing"], &[]),
                (&[], &["smoke", "slow", "wip"]),
            ] {
                let f = filter(include, exclude);
                assert_eq!(any_matching_leaf(&tree(), &f), !matching_leaves(&tree(), &f).is_empty());
            }
            assert!(!any_matching_leaf(&tree(), &filter(&["nothing"], &[])));
            assert!(any_matching_leaf(&scenario(1, "x", &[]), &no_filter()));
        }

        #[test]
        fn matching_leaves_from_a_leaf_returns_that_leaf_only_when_it_matches() {
            let leaf = scenario(1, "x", &["smoke"]);
            assert_eq!(matching_leaves(&leaf, &filter(&["smoke"], &[])), vec![&leaf]);
            assert_eq!(matching_leaves(&leaf, &filter(&["wip"], &[])), Vec::<&TestNode>::new());
        }
    }

    mod chips {
        use super::*;

        #[test]
        fn project_tags_is_sorted_and_unique() {
            assert_eq!(project_tags(&tree()), ["slow", "smoke", "wip"]);
        }

        #[test]
        fn filterable_accepts_only_tags_t_can_select() {
            for t in ["smoke", "owner=bob", "d\"x", "r)x", "a@b", "a~b"] {
                assert!(filterable(t), "{t}");
            }
            // Karate reads `\` as a JS escape: `b\` fails to parse, `a\nb` never matches itself.
            for t in [
                "",
                "a b",
                "a\tb",
                "owner=bob,alice",
                "q'x",
                "p(x",
                "@x",
                "~x",
                "b\\x",
                "b\\",
            ] {
                assert!(!filterable(t), "{t}");
            }
        }

        #[test]
        fn project_tags_skips_tags_t_cannot_select() {
            let t = node(
                NodeKind::Folder,
                "d:",
                "root",
                &[],
                vec![scenario(
                    1,
                    "odd",
                    &["smoke", "owner=bob,alice", "q'x", "p(x", "owner=bob"],
                )],
            );
            assert_eq!(project_tags(&t), ["owner=bob", "smoke"]);
        }

        #[test]
        fn chip_tags_keeps_a_filtered_tag_the_tree_lost() {
            assert!(chip_tags(Some(&tree()), &filter(&["gone"], &[])).contains(&"gone".to_string()));
            assert_eq!(chip_tags(None, &filter(&["b"], &["a"])), ["a", "b"]);
        }
    }

    mod filter_state {
        use super::*;

        #[test]
        fn cycle_tag_goes_off_include_exclude_off() {
            let a = cycle_tag(no_filter(), "smoke");
            assert_eq!(a, filter(&["smoke"], &[]));
            let b = cycle_tag(a, "smoke");
            assert_eq!(b, filter(&[], &["smoke"]));
            let c = cycle_tag(b, "smoke");
            assert_eq!(c, no_filter());
            assert_eq!(no_filter(), filter(&[], &[]));
        }

        #[test]
        fn prune_filter_drops_unknown_tags_and_keeps_hide_non_matching() {
            let f = FilterState {
                hide_non_matching: true,
                ..filter(&["a", "gone"], &["b", "lost"])
            };
            let known = ["a".to_string(), "b".to_string()];
            assert_eq!(
                prune_filter(f, &known),
                FilterState {
                    hide_non_matching: true,
                    ..filter(&["a"], &["b"])
                }
            );
        }

        #[test]
        fn parse_filter_state_rejects_bad_shapes() {
            let ok = json!({ "include": ["a"], "exclude": ["b"], "hideNonMatching": true });
            assert_eq!(
                parse_filter_state(&ok),
                Some(FilterState {
                    hide_non_matching: true,
                    ..filter(&["a"], &["b"])
                })
            );
            let with = |key: &str, value: Value| {
                let mut x = ok.clone();
                x[key] = value;
                x
            };
            let many: Vec<String> = (0..501).map(|i| format!("t{i}")).collect();
            let bad = [
                // JavaScript's `undefined` has no JSON value: `null` stands for both of them
                Value::Null,
                Value::Null,
                json!({}),
                with("include", json!("a")),
                with("include", json!([1])),
                with("include", json!([""])),
                with("include", json!(["x".repeat(201)])),
                with("include", json!(many)),
                {
                    let mut x = with("include", json!(["a"]));
                    x["exclude"] = json!(["a"]);
                    x
                },
                with("hideNonMatching", json!("yes")),
                with("include", json!(["a,b"])),
                with("exclude", json!(["q'x"])),
                with("exclude", json!(["p(x"])),
                with("include", json!(["@a"])),
                with("include", json!(["a b"])),
            ];
            for x in bad {
                assert_eq!(parse_filter_state(&x), None, "{x}");
            }
        }
    }

    /// Not in the TypeScript: the limits themselves are accepted, and a length counts UTF-16 units.
    #[test]
    fn parse_filter_state_accepts_the_limits_and_counts_utf16_units() {
        let ok = |include: Value| json!({ "include": include, "exclude": [], "hideNonMatching": false });
        let many: Vec<String> = (0..500).map(|i| format!("t{i}")).collect();
        assert!(parse_filter_state(&ok(json!(many))).is_some());
        assert!(parse_filter_state(&ok(json!(["x".repeat(200)]))).is_some());
        // 101 emoji are 202 UTF-16 units in JavaScript, though 101 characters and 404 bytes
        assert!(parse_filter_state(&ok(json!(["😀".repeat(101)]))).is_none());
        assert!(parse_filter_state(&ok(json!(["😀".repeat(100)]))).is_some());
    }
}
