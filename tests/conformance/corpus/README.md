# Corpus semantics

The corpus stage unpacks each member's digest-checked source and dependency
archives, then compares semantic index and commit snapshots plus exact inventory
class counts with `expect.json`. The member source is never edited in place.

The Atlas expectation is checked when its private artifacts are provisioned.
Without them the stage emits the coordinator-approved private-unavailable
marker; final milestone acceptance rejects that marker.
