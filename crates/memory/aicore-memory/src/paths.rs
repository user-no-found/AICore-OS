use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPaths {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub projections_dir: PathBuf,
    pub core_md: PathBuf,
    pub status_md: PathBuf,
}

impl MemoryPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let projections_dir = root.join("projections");

        Self {
            db_path: root.join("memory.db"),
            core_md: projections_dir.join("CORE.md"),
            status_md: projections_dir.join("STATUS.md"),
            projections_dir,
            root,
        }
    }
}
