# Data

This folder holds datasets used during development, testing, and validation.

The goal is to keep example and reference data organized from the start so the
parser, transform code, and later solver work can be tested against known
inputs.

## Layout

```text
data/
├── README.md
├── examples/
├── reference/
└── synthetic/
```

## Intended use

### `examples/`

Small, human-readable files used while developing parsers and command-line
workflows.

Good candidates:

- tiny `q,I` CSV files
- tiny `q,I,sigma` CSV files
- intentionally invalid files for parser validation tests

### `synthetic/`

Generated datasets used for controlled numerical testing.

These are useful when the expected behavior is known in advance and we want to
check whether fitting and reporting behave sensibly.

### `reference/`

Real or trusted benchmark datasets used for higher-level validation.

As this folder grows, it is helpful to include a short note near each dataset
about:

- where it came from
- what columns and units it uses
- what it is intended to test

## Notes

- Keep files small where possible, especially in `examples/`
- Prefer plain text formats such as `.csv`, `.dat`, or `.txt`
- Add provenance notes for real datasets when available
- Avoid committing large raw collections unless they are clearly valuable for
  testing or benchmarking
