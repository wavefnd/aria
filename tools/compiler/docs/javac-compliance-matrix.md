# AriaJDK Compiler Compliance Matrix (Java 17 Baseline)

This matrix tracks Aria compiler conformance against Java SE 17 / `javac` behavior targets.

## Scope

- Product: `tools/compiler` (`aria-javac`)
- Baseline level: Java 17 (`--release 17`, `-source 17`, `-target 17`)
- Goal: deterministic `javac`-compatible CLI and diagnostics, then language/bytecode parity

## Status Key

- `PASS`: implemented and covered by automated tests
- `PARTIAL`: implemented subset, explicit limits remain
- `FAIL`: not implemented yet

## CLI And Tooling Compatibility

| Area | Target | Status | Evidence |
|---|---|---|---|
| Version output | `-version` behavior | PASS | `tests/golden_cli.rs::version_output_matches_golden` |
| Help output | `--help/-help/-h/-?` behavior | PASS | `tests/golden_cli.rs::help_output_matches_golden` |
| Unsupported flag handling | fail fast with clear error | PASS | `tests/golden_cli.rs::unsupported_option_stderr_matches_golden` |
| Java level enforcement | reject non-17 on `--release/-source/-target` | PASS | `tests/golden_cli.rs::release_mismatch_stderr_matches_golden` |
| Parse diagnostic formatting | `path:line:col: error: ...` | PASS | `tests/golden_cli.rs::parse_error_diagnostic_matches_golden` |
| Full javac option surface | full parity | FAIL | pending |

## Language Frontend Compatibility

| Area | Status | Notes |
|---|---|---|
| Class/method/field declarations | PASS | basic structure supported |
| Statements (`if`, `while`, `return`, local var) | PASS | covered by backend driver tests |
| Expressions (`arith`, `logic`, comparisons) | PASS | int/boolean path |
| Method call resolution | PARTIAL | overload by arity only; static/instance supported |
| Object creation (`new`) | PARTIAL | currently `new Type()` only (no ctor args) |
| Packages/imports | FAIL | pending |
| Generics | FAIL | pending |
| Exceptions/try-catch | FAIL | pending |
| Lambdas/method refs | FAIL | pending |

## Bytecode And JVM Verification

| Area | Status | Notes |
|---|---|---|
| Class file major version | PASS | emits `61` (Java 17) |
| StackMapTable emission | PASS | required control-flow frames emitted |
| Control-flow verifier safety | PARTIAL | current patterns pass tested paths; broader CFG still pending |
| Constructor model | PARTIAL | default `<init>()V` auto-emitted; custom ctor path pending |

## Certification Readiness Gates (Oracle/JCK-aligned)

1. CLI parity gate: option semantics + exit codes + diagnostics
2. Language gate: Java 17 grammar/type-system coverage
3. Classfile gate: verifier-safe bytecode for all supported constructs
4. Regression gate: golden tests + compatibility corpus in CI
5. Conformance gate: JCK/TCK execution with tracked exclusions

Current readiness: **pre-gate** (foundational compiler path active, major feature gaps remain).

## Immediate Next Work

1. Constructor declarations and `new Type(args)` lowering (`invokespecial` with descriptors)
2. Field assignment and static field access/write
3. Package/import resolution and cross-file symbol model
4. `javac` option compatibility expansion with golden coverage
