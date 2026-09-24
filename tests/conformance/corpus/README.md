# Corpus semantics

The corpus stage adds this manifest to a temporary checkout of each pinned
member, then compares the snapshot kind/object format and exact inventory class
counts with `expect.json`. The member source is never edited in place.

The Atlas expectation is checked when its private artifacts are provisioned.
Without them the stage emits the coordinator-approved private-unavailable
marker; final milestone acceptance rejects that marker.
