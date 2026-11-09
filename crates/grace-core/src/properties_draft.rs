//! The Properties dialog's draft (`PropertiesDialog.tsx`): the rows being edited, the environments that have a list,
//! and the rules that block Save. Electron Grace kept it in the component's signals and store; here it is a plain
//! value the view owns, so its rules are tested without a window (plan, Redesigns). Save sends [`Draft::edit`]; the
//! view drops the draft on close, so a typed secret does not linger.
use std::collections::{HashMap, HashSet};

use crate::import_plan::{Existing, PlanRow};
use crate::model::env::{
    EnvView, MAX_ENVS, MAX_ROWS, PropertiesEdit, RowEdit, RowView, SecretState, name_error, name_keys,
    property_value_error, valid_env_name,
};

/// A row being edited. Scope `""` is "All environments".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: u64,
    pub scope: String,
    pub name: String,
    pub value: String,
    pub secret: bool,
    /// Set on Karate's environment rather than passed as `-D`.
    pub env_var: bool,
    /// Set while the row still stands for a saved secret, whose value only the runner holds. Replacing, renaming or
    /// unticking Secret ends it: the row then needs a value of its own.
    pub saved: Option<SecretState>,
}

impl Row {
    fn forget(&mut self) {
        self.saved = None; // the runner keeps the old value only for an untouched row
        self.value.clear();
    }

    /// A new secret too short for the console to hide it.
    pub fn short_secret(&self) -> bool {
        let n = self.value.chars().count();
        self.secret && self.saved.is_none() && n > 0 && n < 4
    }
}

/// Why an environment cannot be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvError {
    Invalid,
    TooMany,
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvError::Invalid => f.write_str(crate::model::env::INVALID_ENV),
            EnvError::TooMany => write!(f, "At most {MAX_ENVS} environments"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    rows: Vec<Row>,
    /// Environments that have a list in the draft.
    lists: Vec<String>,
    /// The list shown.
    pub scope: String,
    /// The environment chosen in the toolbar: listed even before it has a list.
    env: Option<String>,
    next_id: u64,
}

impl Draft {
    /// The draft of the lists in force.
    pub fn new(view: &EnvView) -> Self {
        let mut d = Draft {
            env: view.env.clone(),
            ..Draft::default()
        };
        d.push_rows("", &view.properties.shared);
        for (env, rows) in &view.properties.by_env {
            d.lists.push(env.clone());
            d.push_rows(env, rows);
        }
        d
    }

    fn push_rows(&mut self, scope: &str, list: &[RowView]) {
        for r in list {
            let row = match r {
                RowView::Plain(p) => Row {
                    id: 0,
                    scope: scope.into(),
                    name: p.name.clone(),
                    value: p.value.clone(),
                    secret: false,
                    env_var: p.env_var == Some(true),
                    saved: None,
                },
                RowView::Secret(s) => Row {
                    id: 0,
                    scope: scope.into(),
                    name: s.name.clone(),
                    value: String::new(),
                    secret: true,
                    env_var: s.env_var == Some(true),
                    saved: Some(s.state),
                },
            };
            self.push(row);
        }
    }

    fn push(&mut self, mut row: Row) -> u64 {
        let id = self.next_id;
        row.id = id;
        self.next_id += 1;
        self.rows.push(row);
        id
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn row(&self, id: u64) -> Option<&Row> {
        self.rows.iter().find(|r| r.id == id)
    }

    pub fn lists(&self) -> &[String] {
        &self.lists
    }

    /// "All environments", the lists, then the chosen environment if it has none yet.
    pub fn scopes(&self) -> Vec<String> {
        let mut s = vec![String::new()];
        s.extend(self.lists.iter().cloned());
        if let Some(env) = &self.env
            && !self.lists.contains(env)
        {
            s.push(env.clone());
        }
        s
    }

    /// The rows of the list shown.
    pub fn visible(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|r| r.scope == self.scope)
    }

    /// Each row's error, by id: too many rows, its name, a duplicate, then its value.
    pub fn errors(&self) -> HashMap<u64, String> {
        let keys = |r: &Row| {
            name_keys(&r.name, r.env_var)
                .into_iter()
                .map(|k| format!("{}\0{k}", r.scope))
                .collect::<Vec<_>>()
        };
        let mut count: HashMap<String, usize> = HashMap::new();
        for r in &self.rows {
            for k in keys(r) {
                *count.entry(k).or_default() += 1;
            }
        }
        let mut place: HashMap<&str, usize> = HashMap::new(); // rows of each list so far
        let mut out = HashMap::new();
        for r in &self.rows {
            let n = place.entry(&r.scope).or_default();
            *n += 1;
            let error = if *n > MAX_ROWS {
                Some(format!("At most {MAX_ROWS} properties per list"))
            } else if r.name.is_empty() {
                Some("Enter a name".into())
            } else if let Some(e) = name_error(&r.name, r.env_var) {
                Some(e.to_string())
            } else if keys(r).iter().any(|k| count[k] > 1) {
                Some("Duplicate name".into())
            } else if r.saved.is_some() {
                None
            } else if r.secret && r.value.is_empty() {
                Some("Enter the secret value".into())
            } else {
                property_value_error(&r.value).map(|e| e.to_string())
            };
            if let Some(e) = error {
                out.insert(r.id, e);
            }
        }
        out
    }

    /// The scopes with a row in error, marked in the list of environments.
    pub fn invalid_scopes(&self) -> HashSet<String> {
        let errors = self.errors();
        self.rows
            .iter()
            .filter(|r| errors.contains_key(&r.id))
            .map(|r| r.scope.clone())
            .collect()
    }

    pub fn has_secret(&self) -> bool {
        self.rows.iter().any(|r| r.secret)
    }

    fn edit_row(&mut self, id: u64, change: impl FnOnce(&mut Row)) {
        if let Some(r) = self.rows.iter_mut().find(|r| r.id == id) {
            change(r);
        }
    }

    pub fn set_name(&mut self, id: u64, name: &str) {
        self.edit_row(id, |r| {
            r.name = name.into();
            if r.saved.is_some() {
                r.forget(); // the runner refuses `keep` across a rename
            }
        });
    }

    pub fn set_value(&mut self, id: u64, value: &str) {
        self.edit_row(id, |r| r.value = value.into());
    }

    /// The value stays (R1.2), and so does a saved secret.
    pub fn set_env_var(&mut self, id: u64, on: bool) {
        self.edit_row(id, |r| r.env_var = on);
    }

    pub fn set_secret(&mut self, id: u64, on: bool) {
        self.edit_row(id, |r| {
            r.secret = on;
            if !on && r.saved.is_some() {
                r.forget();
            }
        });
    }

    /// Replace: the saved secret's row asks for a new value.
    pub fn replace(&mut self, id: u64) {
        self.edit_row(id, Row::forget);
    }

    pub fn delete_row(&mut self, id: u64) {
        self.rows.retain(|r| r.id != id);
    }

    /// Gives `name` a list in the draft, unless the runner would refuse one more.
    fn ensure_list(&mut self, name: &str) -> Result<(), EnvError> {
        if self.lists.iter().any(|l| l == name) {
            return Ok(());
        }
        if self.lists.len() >= MAX_ENVS {
            return Err(EnvError::TooMany);
        }
        self.lists.push(name.into());
        Ok(())
    }

    /// Adds the environment typed (trimmed) and shows its list.
    pub fn add_env(&mut self, typed: &str) -> Result<(), EnvError> {
        let name = typed.trim();
        if !valid_env_name(name) {
            return Err(EnvError::Invalid);
        }
        self.ensure_list(name)?;
        self.scope = name.into();
        Ok(())
    }

    /// Whether the list shown is an environment's own and can be deleted.
    pub fn can_delete_env(&self) -> bool {
        !self.scope.is_empty() && self.lists.contains(&self.scope)
    }

    /// Deletes the list shown and its rows, then shows "All environments".
    pub fn delete_env(&mut self) {
        let gone = std::mem::take(&mut self.scope);
        self.rows.retain(|r| r.scope != gone);
        self.lists.retain(|l| *l != gone);
    }

    /// Adds an empty row to the list shown; the chosen environment gets its list with its first row.
    pub fn add_row(&mut self) -> Result<u64, EnvError> {
        if !self.scope.is_empty() {
            let scope = self.scope.clone();
            self.ensure_list(&scope)?;
        }
        Ok(self.push(Row {
            id: 0,
            scope: self.scope.clone(),
            name: String::new(),
            value: String::new(),
            secret: false,
            env_var: false,
            saved: None,
        }))
    }

    /// The rows of a list, as the import step matches them.
    pub fn existing(&self, scope: &str) -> Vec<Existing> {
        self.rows
            .iter()
            .filter(|r| r.scope == scope)
            .map(|r| Existing {
                name: r.name.clone(),
                env_var: r.env_var,
            })
            .collect()
    }

    /// False when `scope` has no list and the runner would refuse one more.
    pub fn can_add_list(&self, scope: &str) -> bool {
        scope.is_empty() || self.lists.iter().any(|l| l == scope) || self.lists.len() < MAX_ENVS
    }

    /// Adds secret rows of the kind to `target`, replacing in place a row of the same name (R5.4), and shows it.
    pub fn import(&mut self, target: &str, env_var: bool, found: &[PlanRow]) -> Result<(), EnvError> {
        if !target.is_empty() {
            self.ensure_list(target)?;
        }
        for PlanRow { name, value, .. } in found {
            let keys = name_keys(name, env_var);
            let hit = self
                .rows
                .iter_mut()
                .find(|r| r.scope == target && name_keys(&r.name, r.env_var).iter().any(|k| keys.contains(k)));
            match hit {
                // a saved secret's value is dropped for the new one
                Some(r) => {
                    r.name = name.clone();
                    r.value = value.clone();
                    r.secret = true;
                    r.env_var = env_var;
                    r.saved = None;
                }
                None => {
                    self.push(Row {
                        id: 0,
                        scope: target.into(),
                        name: name.clone(),
                        value: value.clone(),
                        secret: true,
                        env_var,
                        saved: None,
                    });
                }
            }
        }
        self.scope = target.into();
        Ok(())
    }

    /// What Save sends: an untouched saved secret as `Keep`, every other row with its value.
    pub fn edit(&self) -> PropertiesEdit {
        let of = |s: &str| {
            self.rows
                .iter()
                .filter(|r| r.scope == s)
                .map(|r| {
                    let env_var = r.env_var.then_some(true);
                    if r.saved.is_some() {
                        RowEdit::Keep {
                            name: r.name.clone(),
                            env_var,
                        }
                    } else {
                        RowEdit::Value {
                            name: r.name.clone(),
                            value: r.value.clone(),
                            secret: r.secret,
                            env_var,
                        }
                    }
                })
                .collect::<Vec<_>>()
        };
        PropertiesEdit {
            shared: of(""),
            by_env: self.lists.iter().map(|e| (e.clone(), of(e))).collect(),
        }
    }
}

#[cfg(test)]
mod tests;
