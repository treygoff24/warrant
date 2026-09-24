# Colocated test: break the control

Hard control: a helper below `src/__tests__` remains a test even though the app
module owns all of `src/**`. This is adapted from Specgate's test-helper-leak
fixture and turns red if ownership overwrites classification.
