# Snapshot exclusions: negative boundary

Hard control: ignore rules affect untracked paths only. `tracked.log` is forced
into the baseline commit, so adding `*.log` to `.gitignore` must not remove it.
