# Snapshot exclusions: break the control

Hard controls: ignored untracked files are counted; oversize and external
symlink entries are unread with explicit reasons; submodules are recorded and
not descended. Clearing `summary.unread` is the deliberate runner mutation.
The later `unread-in-scope` facet proof belongs to W2.1.
