# Unowned source: break the control

Hard control: a first-party file added outside every module selector must stay
in `summary.unowned_source`. Clearing that field is the runner's deliberate red
mutation and models disabling the unowned-source check.
