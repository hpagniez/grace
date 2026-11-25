//! `src/main/karate/maven.ts`: a Maven project's default karate.env, read from its pom.xml
//! (docs/specs/2026-11-04-pom-defaults.md R3).
use super::js;
use crate::model::env::valid_env_name;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Default)]
struct El {
    name: String,
    children: Vec<El>,
    text: String,
}

impl El {
    fn kids<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a El> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// The first child of each name in turn.
    fn at(&self, names: &[&str]) -> Option<&El> {
        names
            .iter()
            .try_fold(self, |el, name| el.children.iter().find(|c| c.name == *name))
    }
}

/// A comment, CDATA, `<?…?>` or `<!…>`, an end tag, a start or empty tag (attributes ignored), text.
static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"<!--[\s\S]*?-->|<!\[CDATA\[([\s\S]*?)\]\]>|<[?!][^>]*>|</\s*([^\s>]+)\s*>|<([^\s/>!?]+)[^>]*?(/)?>|([^<]+)",
    )
    .unwrap()
});

/// No pom nests this deep. Dropping a tree recurses, so a deeper one is refused before it can overflow the stack.
const DEEPEST: usize = 100;

/// The root element, or None when the elements are not well nested.
fn element(xml: &str) -> Option<El> {
    // the elements still open, under a nameless one that holds the root
    let mut stack = vec![El::default()];
    for token in TOKEN.captures_iter(xml) {
        if let Some(start) = token.get(3) {
            let el = El {
                name: start.as_str().into(),
                ..El::default()
            };
            if token.get(4).is_some() {
                stack.last_mut()?.children.push(el);
            } else if stack.len() < DEEPEST {
                stack.push(el);
            } else {
                return None;
            }
        } else if let Some(end) = token.get(2) {
            let el = stack.pop()?;
            // `?` when it was the nameless one
            let parent = stack.last_mut()?;
            if el.name != end.as_str() {
                return None;
            }
            parent.children.push(el);
        } else if let Some(text) = token.get(1).or_else(|| token.get(5)) {
            stack.last_mut()?.text.push_str(text.as_str());
        }
    }
    if stack.len() != 1 {
        return None;
    }
    stack.pop()?.children.into_iter().next()
}

const PLUGINS: [&str; 2] = ["maven-surefire-plugin", "maven-failsafe-plugin"];

/// R3: Surefire's, then Failsafe's `karate.env` entry, else the `karate.env` property; None if none Grace takes. Never
/// fails.
pub fn maven_env(root: &Path) -> Option<String> {
    let bytes = fs::read(root.join("pom.xml")).ok()?;
    let project = element(&String::from_utf8_lossy(&bytes)).filter(|el| el.name == "project")?;
    let props: HashMap<&str, &str> = project
        .at(&["properties"])
        .map(|p| {
            p.children
                .iter()
                .map(|p| (p.name.as_str(), js::trim(&p.text)))
                .collect()
        })
        .unwrap_or_default();
    let lists = [
        project.at(&["build", "plugins"]),
        project.at(&["build", "pluginManagement", "plugins"]),
    ];
    let entry = PLUGINS
        .iter()
        .flat_map(|id| {
            lists.iter().flatten().flat_map(move |list| {
                list.kids("plugin")
                    .filter(move |p| p.at(&["artifactId"]).is_some_and(|a| js::trim(&a.text) == *id))
            })
        })
        .find_map(|p| p.at(&["configuration", "systemPropertyVariables", "karate.env"]));
    let raw = match entry {
        Some(entry) => js::trim(&entry.text),
        None => props.get("karate.env")?,
    };
    if raw.matches("${").count() > 1 {
        return None;
    }
    let reference = raw
        .split_once("${")
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(name, _)| name);
    let value = match reference {
        Some(name) => raw.replacen(&format!("${{{name}}}"), props.get(name)?, 1),
        None => raw.to_string(),
    };
    let value = js::trim(&value);
    (!value.contains("${") && valid_env_name(value)).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// A project folder, with no pom.xml.
    fn empty() -> TempDir {
        tempfile::Builder::new().prefix("kk-maven-").tempdir().unwrap()
    }

    /// A project folder whose pom.xml holds `inner` inside `<project>`.
    fn project(inner: &str) -> TempDir {
        let d = empty();
        let pom = format!("<?xml version=\"1.0\"?>\n<project>{inner}</project>\n");
        fs::write(d.path().join("pom.xml"), pom).unwrap();
        d
    }

    fn env_of(inner: &str) -> Option<String> {
        maven_env(project(inner).path())
    }

    fn some(env: &str) -> Option<String> {
        Some(env.into())
    }

    fn property(name: &str, value: &str) -> String {
        format!("<properties><{name}>{value}</{name}></properties>")
    }

    fn plugin(id: &str, entry: &str) -> String {
        format!(
            "<plugin><groupId>org.apache.maven.plugins</groupId><artifactId>{id}</artifactId>\
             <configuration><systemPropertyVariables><karate.env>{entry}</karate.env></systemPropertyVariables>\
             </configuration></plugin>"
        )
    }

    /// Surefire's entry, in `<build><plugins>`.
    fn surefire(entry: &str) -> String {
        format!(
            "<build><plugins>{}</plugins></build>",
            plugin("maven-surefire-plugin", entry)
        )
    }

    #[test]
    fn takes_the_karate_env_property_alone() {
        assert_eq!(env_of(&property("karate.env", "dev")), some("dev"));
    }

    #[test]
    fn takes_surefires_entry_as_written() {
        assert_eq!(env_of(&surefire("qa")), some("qa"));
    }

    #[test]
    fn resolves_surefires_karate_env_from_the_properties() {
        let pom = property("karate.env", "dev") + &surefire("${karate.env}");
        assert_eq!(env_of(&pom), some("dev"));
    }

    #[test]
    fn resolves_one_reference_inside_other_text() {
        let pom = property("region", "eu") + &surefire("qa-${region}");
        assert_eq!(env_of(&pom), some("qa-eu"));
    }

    #[test]
    fn gives_none_when_surefires_property_is_missing_without_falling_back() {
        let pom = property("karate.env", "dev") + &surefire("${missing}");
        assert_eq!(env_of(&pom), None);
    }

    #[test]
    fn takes_failsafes_entry() {
        let failsafe = format!(
            "<build><plugins>{}</plugins></build>",
            plugin("maven-failsafe-plugin", "${karate.env}")
        );
        assert_eq!(env_of(&(property("karate.env", "dev") + &failsafe)), some("dev"));
    }

    #[test]
    fn takes_surefires_entry_in_plugin_management() {
        let pom = format!(
            "<build><pluginManagement><plugins>{}</plugins></pluginManagement></build>",
            plugin("maven-surefire-plugin", "qa")
        );
        assert_eq!(env_of(&pom), some("qa"));
    }

    #[test]
    fn puts_surefire_before_failsafe() {
        let both = format!(
            "<build><plugins>{}{}</plugins></build>",
            plugin("maven-failsafe-plugin", "uat"),
            plugin("maven-surefire-plugin", "qa")
        );
        assert_eq!(env_of(&both), some("qa"));
    }

    #[test]
    fn leaves_out_an_executions_configuration() {
        let execution = "<build><plugins><plugin><artifactId>maven-surefire-plugin</artifactId><executions><execution>\
             <configuration><systemPropertyVariables><karate.env>qa</karate.env></systemPropertyVariables>\
             </configuration></execution></executions></plugin></plugins></build>";
        assert_eq!(env_of(execution), None);
    }

    #[test]
    fn leaves_out_a_profiles_properties() {
        let pom = format!(
            "<profiles><profile>{}</profile></profiles>",
            property("karate.env", "dev")
        );
        assert_eq!(env_of(&pom), None);
    }

    #[test]
    fn leaves_out_a_comment() {
        let pom = format!("<!-- {} -->", property("karate.env", "dev"));
        assert_eq!(env_of(&pom), None);
    }

    #[test]
    fn gives_none_for_a_name_grace_does_not_take() {
        assert_eq!(env_of(&property("karate.env", "x y")), None);
    }

    #[test]
    fn gives_none_when_elements_are_not_well_nested() {
        assert_eq!(env_of("<properties><karate.env>dev</properties></karate.env>"), None);
    }

    #[test]
    fn gives_none_for_more_than_one_reference() {
        let pom = property("a", "x") + &property("b", "y") + &surefire("${a}${b}");
        assert_eq!(env_of(&pom), None);
    }

    #[test]
    fn gives_none_when_the_resolved_value_still_holds_a_reference() {
        let pom = property("karate.env", "${other}") + &surefire("${karate.env}");
        assert_eq!(env_of(&pom), None);
    }

    #[test]
    fn lets_an_empty_surefire_entry_decide() {
        let pom = property("karate.env", "dev") + &surefire("");
        assert_eq!(env_of(&pom), None);
    }

    #[test]
    fn reads_a_namespaced_project_with_a_bom_and_windows_line_ends() {
        let d = empty();
        let pom = "\u{FEFF}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
             <project xmlns=\"http://maven.apache.org/POM/4.0.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\r\n  \
             <properties>\r\n    <karate.env>dev</karate.env>\r\n  </properties>\r\n</project>\r\n";
        fs::write(d.path().join("pom.xml"), pom).unwrap();
        assert_eq!(maven_env(d.path()), some("dev"));
    }

    #[test]
    fn reads_cdata_as_text() {
        assert_eq!(env_of(&property("karate.env", "<![CDATA[dev]]>")), some("dev"));
    }

    #[test]
    fn gives_none_without_a_pom_xml() {
        assert_eq!(maven_env(empty().path()), None);
    }

    // Beyond the Vitest file.

    /// Dropping a tree as deep as this would overflow the stack and take Grace with it.
    #[test]
    fn gives_none_for_elements_nested_deeper_than_any_pom() {
        let deep = "<a>".repeat(200_000) + &"</a>".repeat(200_000);
        assert_eq!(env_of(&(property("karate.env", "dev") + &deep)), None);
    }
}
