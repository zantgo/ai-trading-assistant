# RECOMMENDATION — export JSON keys NOT rendered by panel

Only the transport-metadata leaves below are skipped by the panel;
every other export leaf is rendered (builder mirrors panel 1:1, verified
by `exportConsistency.test.ts` which renders the panel and asserts
DOM value == export JSON value per field).

| # | EXPORT JSON KEY | TYPED VALUE |
|---|---|---|
| 1 | `source_tab` | str |
| 2 | `meta.datetime_utc` | str |
| 3 | `meta.timestamp` | int |
| 4 | `meta.is_completed` | bool |
| 5 | `header.status` | str |
