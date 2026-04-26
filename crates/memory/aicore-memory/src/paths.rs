use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPaths {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub lock_path: PathBuf,
    pub projections_dir: PathBuf,
    pub wiki_dir: PathBuf,
    pub core_md: PathBuf,
    pub status_md: PathBuf,
    pub permanent_md: PathBuf,
    pub decisions_md: PathBuf,
    pub wiki_index_md: PathBuf,
    pub wiki_core_md: PathBuf,
    pub wiki_decisions_md: PathBuf,
    pub wiki_status_md: PathBuf,
}

impl MemoryPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let projections_dir = root.join("projections");
        let wiki_dir = root.join("wiki");

        Self {
            db_path: root.join("memory.db"),
            lock_path: root.join("memory.lock"),
            core_md: projections_dir.join("CORE.md"),
            status_md: projections_dir.join("STATUS.md"),
            permanent_md: projections_dir.join("PERMANENT.md"),
            decisions_md: projections_dir.join("DECISIONS.md"),
            wiki_index_md: wiki_dir.join("index.md"),
            wiki_core_md: wiki_dir.join("core.md"),
            wiki_decisions_md: wiki_dir.join("decisions.md"),
            wiki_status_md: wiki_dir.join("status.md"),
            projections_dir,
            wiki_dir,
            root,
        }
    }
}
