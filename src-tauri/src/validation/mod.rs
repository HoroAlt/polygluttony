//! Translation validation pipeline.
//!
//! Ports the Python `validation/` package: structure and marker integrity checks
//! (`<0001:D>` line markers, fuzzy recovery, partial-success fallback) plus a
//! five-signal weighted drift detector (punctuation, glossary position, sentence
//! type, last line, length ratio).
//!
//! Planned submodules: `alignment`, `line_marker`, `marker_result`,
//! `structure_result`, `drift_detector`, `drift_result`, and `signals/`
//! (`punctuation`, `glossary_position`, `sentence_type`, `last_line`,
//! `length_ratio`).
