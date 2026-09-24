# Generated drift: absent output

A glob declaration with one captured match whose producer also writes an output
the snapshot lacks. Discovery sees a match and records no absence; verification
reports `gen/b.ts` as absent. An absence is not drift: exit 0, no drift rows.
