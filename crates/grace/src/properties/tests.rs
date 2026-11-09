//! `PropertiesDialog.test.tsx`'s view tests: what the dialog shows and what its controls do. Its rules of the draft
//! (errors, what Save sends) are `grace_core::properties_draft::tests`; the dialog's frame, opening and Escape are
//! GPUI Kit's, tested end to end (`grace-e2e/tests/properties.rs`).
use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::testing::{Harness, open_with, project};
use gpui::{Modifiers, TestAppContext, VisualTestContext};
use grace_core::model::env::{PlainRow, PropertyListsView, RowEdit, SecretRowView};
use grace_core::model::node::Layout;
use grace_core::state::AppState;

fn plain(name: &str, value: &str) -> grace_core::model::env::RowView {
    grace_core::model::env::RowView::Plain(PlainRow {
        name: name.into(),
        value: value.into(),
        env_var: None,
    })
}

fn secret(name: &str, state: SecretState) -> grace_core::model::env::RowView {
    grace_core::model::env::RowView::Secret(SecretRowView {
        name: name.into(),
        secret: true,
        state,
        env_var: None,
    })
}

/// `saved` of the TypeScript: dev chosen, lists qa and prod.
fn saved() -> EnvView {
    EnvView {
        env: Some("dev".into()),
        suggestions: vec![],
        overlays: vec![],
        properties: PropertyListsView {
            shared: vec![plain("timeout", "5000")],
            by_env: [
                (
                    "qa".to_string(),
                    vec![plain("timeout", "9000"), secret("api.token", SecretState::Set)],
                ),
                ("prod".to_string(), vec![]),
            ]
            .into_iter()
            .collect(),
        },
        secrets_available: true,
    }
}

fn state(env: EnvView) -> AppState {
    let mut s = project("D:/p", Layout::Standalone, 2);
    s.env = Some(env);
    s
}

/// A selector as GPUI's `debug_bounds` takes it: a row's selectors carry its id, made at run time.
fn leak(id: &str) -> &'static str {
    Box::leak(id.to_string().into_boxed_str())
}

struct Dialog {
    h: Harness,
    view: Entity<PropertiesView>,
    closed: Rc<RefCell<usize>>,
}

fn open(cx: &mut TestAppContext, env: EnvView) -> (Dialog, VisualTestContext) {
    let slot: Rc<RefCell<Option<Entity<PropertiesView>>>> = Rc::default();
    let closed: Rc<RefCell<usize>> = Rc::default();
    let (keep, count) = (slot.clone(), closed.clone());
    let (h, mut cx) = open_with(cx, 1400., state(env.clone()), move |grace, window, cx| {
        let view = cx.new(|cx| PropertiesView::new(grace.clone(), &env, window, cx));
        window
            .subscribe(&view, cx, move |_, _: &Close, _, _| *count.borrow_mut() += 1)
            .detach();
        *keep.borrow_mut() = Some(view.clone());
        Box::new(move |_, _, _| div().p_4().child(view.clone()).into_any_element())
    });
    let view = slot.borrow().clone().unwrap();
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (Dialog { h, view, closed }, cx)
}

impl Dialog {
    fn draft(&self, cx: &mut VisualTestContext) -> Draft {
        cx.update(|_, cx| self.view.read(cx).draft.clone())
    }

    fn visible(&self, cx: &mut VisualTestContext) -> Vec<Row> {
        self.draft(cx).visible().cloned().collect()
    }

    fn shows(&self, cx: &mut VisualTestContext, id: &str) -> bool {
        cx.debug_bounds(leak(id)).is_some()
    }

    fn click(&self, cx: &mut VisualTestContext, id: &str) {
        let at = cx
            .debug_bounds(leak(id))
            .unwrap_or_else(|| panic!("{id} is not drawn"))
            .center();
        cx.simulate_click(at, Modifiers::default());
        cx.run_until_parked();
    }

    /// Clicks into a box and types `text` over what it holds.
    fn type_in(&self, cx: &mut VisualTestContext, id: &str, text: &str) {
        self.click(cx, id);
        cx.simulate_keystrokes("ctrl-a");
        if text.is_empty() {
            cx.simulate_keystrokes("backspace");
        } else {
            cx.simulate_input(text);
        }
        cx.run_until_parked();
    }

    fn text(&self, cx: &mut VisualTestContext, id: u64, value: bool) -> String {
        cx.update(|_, cx| {
            let f = &self.view.read(cx).fields[&id];
            let input = if value { &f.value } else { &f.name };
            input.read(cx).value().to_string()
        })
    }

    fn focused(&self, cx: &mut VisualTestContext, id: u64, value: bool) -> bool {
        cx.update(|window, cx| {
            let f = &self.view.read(cx).fields[&id];
            let input = if value { &f.value } else { &f.name };
            input.read(cx).focus_handle(cx).is_focused(window)
        })
    }

    fn closed(&self) -> usize {
        *self.closed.borrow()
    }
}

#[gpui::test]
fn lists_scopes_and_rows_and_shows_the_chosen_ones(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    for i in 0..4 {
        assert!(
            d.shows(&mut cx, &format!("scope-item-{i}")),
            "All environments, qa, prod, dev"
        );
    }
    let rows = d.visible(&mut cx);
    assert_eq!(rows.len(), 1);
    assert_eq!(d.text(&mut cx, rows[0].id, false), "timeout");
    assert_eq!(d.text(&mut cx, rows[0].id, true), "5000");
    assert!(!d.shows(&mut cx, &format!("row-error-{}", rows[0].id)));
    d.click(&mut cx, "scope-item-1");
    let qa = d.visible(&mut cx);
    assert_eq!(
        qa.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["timeout", "api.token"]
    );
    // a saved secret shows dots and Replace, never a value box
    assert!(d.shows(&mut cx, &format!("prop-dots-{}", qa[1].id)));
    assert!(d.shows(&mut cx, &format!("prop-replace-{}", qa[1].id)));
    assert!(!d.shows(&mut cx, &format!("prop-value-{}", qa[1].id)));
}

#[gpui::test]
fn a_new_row_takes_the_focus_and_says_what_it_lacks(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "add-property");
    let row = d.visible(&mut cx).last().unwrap().id;
    assert!(d.focused(&mut cx, row, false));
    assert!(d.shows(&mut cx, &format!("row-error-{row}")), "Enter a name");
    cx.simulate_input("karate.env");
    cx.run_until_parked();
    assert_eq!(d.draft(&mut cx).row(row).unwrap().name, "karate.env");
    assert_eq!(d.draft(&mut cx).errors()[&row], "Reserved name");
    d.type_in(&mut cx, &format!("prop-name-{row}"), "my.own");
    assert!(!d.shows(&mut cx, &format!("row-error-{row}")));
    d.click(&mut cx, &format!("prop-delete-{row}"));
    assert_eq!(d.visible(&mut cx).len(), 1);
}

#[gpui::test]
fn replace_empties_and_focuses_a_hidden_value_box(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "scope-item-1");
    let token = d.visible(&mut cx)[1].id;
    d.click(&mut cx, &format!("prop-replace-{token}"));
    assert!(d.shows(&mut cx, &format!("prop-value-{token}")));
    assert!(!d.shows(&mut cx, &format!("prop-dots-{token}")));
    assert!(d.focused(&mut cx, token, true));
    assert!(
        cx.update(|_, cx| d.view.read(cx).fields[&token].masked),
        "the typed text is hidden"
    );
    cx.simulate_input("s3cret-token-123");
    cx.run_until_parked();
    assert_eq!(d.draft(&mut cx).row(token).unwrap().value, "s3cret-token-123");
}

#[gpui::test]
fn renaming_a_saved_secret_empties_its_value_box(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "scope-item-1");
    let token = d.visible(&mut cx)[1].id;
    d.type_in(&mut cx, &format!("prop-name-{token}"), "api.token2");
    assert!(d.shows(&mut cx, &format!("prop-value-{token}")));
    assert_eq!(d.text(&mut cx, token, true), "");
    assert!(
        d.shows(&mut cx, &format!("row-error-{token}")),
        "Enter the secret value"
    );
}

#[gpui::test]
fn unticking_secret_shows_the_value_box_unmasked(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "scope-item-1");
    let token = d.visible(&mut cx)[1].id;
    d.click(&mut cx, &format!("prop-secret-{token}"));
    assert!(!d.draft(&mut cx).row(token).unwrap().secret);
    assert!(d.shows(&mut cx, &format!("prop-value-{token}")));
    assert!(!cx.update(|_, cx| d.view.read(cx).fields[&token].masked));
    d.click(&mut cx, &format!("prop-envvar-{token}"));
    assert!(d.draft(&mut cx).row(token).unwrap().env_var);
}

#[gpui::test]
fn adds_and_deletes_an_environment_list(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.type_in(&mut cx, "add-env-input", "not valid!");
    d.click(&mut cx, "add-env");
    assert!(d.shows(&mut cx, "add-env-error"));
    d.type_in(&mut cx, "add-env-input", "staging");
    assert!(!d.shows(&mut cx, "add-env-error"), "typing clears the error");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let draft = d.draft(&mut cx);
    assert_eq!(draft.scope, "staging");
    assert_eq!(draft.scopes(), ["", "qa", "prod", "staging", "dev"]);
    assert_eq!(
        cx.update(|_, cx| d.view.read(cx).new_env.read(cx).value().to_string()),
        ""
    );
    d.click(&mut cx, "delete-env");
    assert_eq!(d.draft(&mut cx).scopes(), ["", "qa", "prod", "dev"]);
}

#[gpui::test]
fn save_sends_the_draft_and_closes_once_applied(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "save-properties");
    let edits = d.h.backend.edits.lock().unwrap().clone();
    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0].by_env["qa"][1],
        RowEdit::Keep {
            name: "api.token".into(),
            env_var: None
        }
    );
    assert_eq!(d.closed(), 1);
}

#[gpui::test]
fn a_refused_save_keeps_the_dialog_and_its_draft(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.h.backend.refuse_save.store(true, std::sync::atomic::Ordering::SeqCst);
    d.click(&mut cx, "add-property");
    cx.simulate_input("kept.row");
    cx.run_until_parked();
    d.click(&mut cx, "save-properties");
    assert_eq!(d.h.backend.edits.lock().unwrap().len(), 1);
    assert_eq!(d.closed(), 0);
    assert!(!cx.update(|_, cx| d.view.read(cx).saving), "Save can be pressed again");
    assert_eq!(d.visible(&mut cx).len(), 2);
}

#[gpui::test]
fn an_error_blocks_save(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "add-property"); // no name
    d.click(&mut cx, "save-properties");
    assert!(d.h.backend.edits.lock().unwrap().is_empty());
    assert_eq!(d.closed(), 0);
}

#[gpui::test]
fn cancel_closes_and_sends_nothing(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "cancel-properties");
    assert_eq!(d.closed(), 1);
    assert!(d.h.backend.edits.lock().unwrap().is_empty());
}

#[gpui::test]
fn the_secrets_note_shows_with_a_secret_and_a_short_one_has_a_hint(cx: &mut TestAppContext) {
    let mut env = saved();
    env.properties.by_env.clear();
    let (d, mut cx) = open(cx, env);
    assert!(!d.shows(&mut cx, "secrets-note"));
    let row = d.visible(&mut cx)[0].id;
    d.click(&mut cx, &format!("prop-secret-{row}"));
    assert!(d.shows(&mut cx, "secrets-note"));
    d.type_in(&mut cx, &format!("prop-value-{row}"), "abc");
    assert!(d.shows(&mut cx, &format!("row-hint-{row}")));
    d.type_in(&mut cx, &format!("prop-value-{row}"), "abcd");
    assert!(!d.shows(&mut cx, &format!("row-hint-{row}")));
}

#[gpui::test]
fn without_secret_storage_nothing_can_become_a_secret(cx: &mut TestAppContext) {
    let mut env = saved();
    env.secrets_available = false;
    let (d, mut cx) = open(cx, env);
    assert!(d.shows(&mut cx, "secrets-unavailable"));
    let row = d.visible(&mut cx)[0].id;
    d.click(&mut cx, &format!("prop-secret-{row}"));
    assert!(!d.draft(&mut cx).row(row).unwrap().secret);
    d.click(&mut cx, "import-file");
    assert!(cx.update(|_, cx| d.view.read(cx).import.is_none()));
}

#[gpui::test]
fn an_unreadable_secret_says_so(cx: &mut TestAppContext) {
    let mut env = saved();
    env.properties.shared = vec![secret("t", SecretState::Unreadable)];
    let (d, mut cx) = open(cx, env);
    let row = d.visible(&mut cx)[0].id;
    assert!(d.shows(&mut cx, &format!("prop-unreadable-{row}")));
    d.click(&mut cx, &format!("prop-replace-{row}"));
    assert!(!d.shows(&mut cx, &format!("prop-unreadable-{row}")));
}

fn properties_file(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("secrets.properties"), text).unwrap();
    dir
}

#[gpui::test]
fn import_shows_what_it_adds_and_puts_secret_variables_in_the_draft(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    let dir = properties_file("db.user=u\ndb.password=p\n9lives=x\nempty=\n");
    *d.h.seams.file.lock().unwrap() = Some(dir.path().join("secrets.properties"));
    d.click(&mut cx, "import-file");
    assert!(d.shows(&mut cx, "import-step"));
    assert!(!d.shows(&mut cx, "add-property"), "the step takes the rows' place");
    assert!(d.shows(&mut cx, "import-skip-0"), "9lives: Invalid name");
    assert!(d.shows(&mut cx, "import-skip-empty"));
    d.click(&mut cx, "import-confirm");
    assert!(!d.shows(&mut cx, "import-step"));
    let draft = d.draft(&mut cx);
    let shared: Vec<_> = draft
        .visible()
        .map(|r| (r.name.as_str(), r.secret, r.env_var))
        .collect();
    assert_eq!(
        shared,
        [
            ("timeout", false, false),
            ("DB_USER", true, true),
            ("DB_PASSWORD", true, true)
        ]
    );
    let focus = cx.update(|window, cx| d.view.read(cx).import_focus.is_focused(window));
    assert!(focus, "back on Import from file…");
}

#[gpui::test]
fn back_changes_nothing_and_no_file_picked_shows_no_step(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    d.click(&mut cx, "import-file");
    assert!(!d.shows(&mut cx, "import-step"), "no file picked");
    let dir = properties_file("a=b\n");
    *d.h.seams.file.lock().unwrap() = Some(dir.path().join("secrets.properties"));
    d.click(&mut cx, "import-file");
    d.click(&mut cx, "import-back");
    assert!(!d.shows(&mut cx, "import-step"));
    assert_eq!(d.visible(&mut cx).len(), 1);
}

#[gpui::test]
fn a_file_that_cannot_be_read_says_why(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    *d.h.seams.file.lock().unwrap() = Some(std::path::PathBuf::from("/no/such/file.properties"));
    d.click(&mut cx, "import-file");
    assert!(d.shows(&mut cx, "import-error"));
}

#[gpui::test]
fn an_invalid_prefix_blocks_the_import(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    let dir = properties_file("a=b\n");
    *d.h.seams.file.lock().unwrap() = Some(dir.path().join("secrets.properties"));
    d.click(&mut cx, "import-file");
    d.type_in(&mut cx, "import-prefix", "my-app");
    assert!(d.shows(&mut cx, "import-prefix-error"));
    assert!(!d.shows(&mut cx, "import-preview"), "no name to show");
    d.click(&mut cx, "import-confirm");
    assert!(d.shows(&mut cx, "import-step"), "Import is disabled");
    d.type_in(&mut cx, "import-prefix", "MY_");
    assert!(!d.shows(&mut cx, "import-prefix-error"));
    d.click(&mut cx, "import-confirm");
    let names: Vec<_> = d.visible(&mut cx).into_iter().map(|r| r.name).collect();
    assert_eq!(names, ["timeout", "MY_A"]);
}

#[gpui::test]
fn an_import_past_the_list_limit_is_blocked(cx: &mut TestAppContext) {
    let (d, mut cx) = open(cx, saved());
    let keys: String = (0..500).map(|i| format!("k{i}=v\n")).collect();
    let dir = properties_file(&keys);
    *d.h.seams.file.lock().unwrap() = Some(dir.path().join("secrets.properties"));
    d.click(&mut cx, "import-file");
    assert!(d.shows(&mut cx, "import-limit"), "500 added beside timeout");
    d.click(&mut cx, "import-confirm");
    assert!(d.shows(&mut cx, "import-step"));
    assert_eq!(d.visible(&mut cx).len(), 1);
}
