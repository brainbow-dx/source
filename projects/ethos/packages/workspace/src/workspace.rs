use crate::Project;
use crate::ProjectKind;
use crate::WorkspaceFs;

/// The well-known subdirectory `Workspace::scan` looks for projects under — matches this
/// monorepo's actual layout (`projects/escher`, `projects/atlas`, ...). Not configurable yet;
/// revisit if a scanned workspace ever needs a different convention.
const PROJECTS_DIR: &str = "projects";

/// A read-only model of one workspace root: which projects exist under it, and what kind of
/// project each looks like. See this crate's own top-level doc comment for why this stays this
/// narrow on day one — no file trees, no dependency graph between projects, and nothing mutable
/// yet, even though the proposal this is built from expects `Workspace` to eventually be "the
/// central point other Ethos actions orchestrate mutations through." Widen it once a real second
/// consumer needs more than "what projects exist."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub root: String,
    pub projects: Vec<Project>,
}

impl Workspace {
    /// Scans `root` (a path in whatever space `fs` reads, e.g. `""` for a `NativeFs` rooted
    /// exactly at the workspace root) for a `projects/` directory and records one `Project` per
    /// direct subdirectory found there. A missing `projects/` directory just means an empty
    /// `Workspace`, not an error — the same "optional, not load-bearing" treatment Anvil's own
    /// `discover_js_commands` gives a missing `commands/` directory.
    pub fn scan(fs: &dyn WorkspaceFs, root: &str) -> Self {
        let projects_dir = join(root, PROJECTS_DIR);

        let projects = fs
            .read_dir(&projects_dir)
            .into_iter()
            .filter(|entry| entry.is_dir)
            .map(|entry| {
                let path = join(&projects_dir, &entry.name);
                let kind = ProjectKind::detect(fs, &path);
                Project { name: entry.name, path, kind }
            })
            .collect();

        Workspace { root: root.to_string(), projects }
    }
}

fn join(root: &str, segment: &str) -> String {
    if root.is_empty() {
        segment.to_string()
    } else {
        format!("{root}/{segment}")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;
    use crate::DirEntry;

    /// A plain in-memory `WorkspaceFs` for testing `Workspace::scan` without touching a real
    /// filesystem — also doubles as a worked example of implementing this trait for a non-native
    /// embedder, which is exactly the case this crate's `fs` abstraction exists for.
    struct FakeFs {
        dirs: HashMap<String, Vec<DirEntry>>,
        files: RefCell<std::collections::HashSet<String>>,
    }

    impl WorkspaceFs for FakeFs {
        fn read_dir(&self, path: &str) -> Vec<DirEntry> {
            self.dirs.get(path).cloned().unwrap_or_default()
        }

        fn is_file(&self, path: &str) -> bool {
            self.files.borrow().contains(path)
        }
    }

    #[test]
    fn scans_projects_and_detects_kind_by_marker_files() {
        let mut dirs = HashMap::new();
        dirs.insert(
            "projects".to_string(),
            vec![
                DirEntry { name: "escher".to_string(), is_dir: true },
                DirEntry { name: "ethos".to_string(), is_dir: true },
                DirEntry { name: ".DS_Store".to_string(), is_dir: false },
            ],
        );
        let files = RefCell::new(std::collections::HashSet::from(["projects/escher/Cargo.toml".to_string()]));
        let fs = FakeFs { dirs, files };

        let workspace = Workspace::scan(&fs, "");

        assert_eq!(workspace.projects.len(), 2);
        let escher = workspace.projects.iter().find(|project| project.name == "escher").unwrap();
        assert!(escher.kind.rust);
        assert!(!escher.kind.deno);
        let ethos = workspace.projects.iter().find(|project| project.name == "ethos").unwrap();
        assert!(ethos.kind.is_unrecognized());
    }

    #[test]
    fn missing_projects_directory_is_an_empty_workspace_not_an_error() {
        let fs = FakeFs { dirs: HashMap::new(), files: RefCell::new(std::collections::HashSet::new()) };
        let workspace = Workspace::scan(&fs, "");
        assert!(workspace.projects.is_empty());
    }
}
