# OpenDefAlgMerging oracle (pinned)

Historical IsoType merging algorithm used as the parity oracle for Rust
`megahit_merge` (`isOpenDef` + `TupleModelHash` + `propagar`).

This is **not** Aut-orbit `iso_merge` from fopy.

## Pin

See `COMMIT.txt` (source revision when vendored). Upstream:
https://github.com/pablogventura/OpenDefAlgMerging

## Run

```bash
python3 main.py /path/to/model.model
```

Prints `DEFINABLE` or `NOT DEFINABLE`.

Parity harness: `scripts/parity_megahit_oracle.py`.
