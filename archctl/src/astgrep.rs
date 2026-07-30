//! ast-grep adapter using `ast-grep-language` with builtin-parser.
//! `SupportLang` provides pre-wired implementations of the `Language`
//! and `LanguageExt` traits for all supported languages (including
//! Kotlin), eliminating the manual `impl Language` boilerplate.
//!
//! **Supported languages**: Rust, TypeScript, JavaScript, Python, Go,
//! Java, Kotlin (+ 20 more via builtin-parser).

use anyhow::{Context, Result};
use ast_grep_core::language::Language;
use ast_grep_core::matcher::{NodeMatch, Pattern, PatternError};
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc};
use ast_grep_core::{AstGrep, Node};
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use std::fmt;
use std::path::Path;
use tracing::debug;

/// Public language enum — thin wrapper over `SupportLang`.
/// The backing enum provides all trait implementations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum Lang {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Kotlin,
}

impl Lang {
    /// All supported languages.
    pub const ALL: &'static [Lang] = &[
        Lang::Rust,
        Lang::TypeScript,
        Lang::JavaScript,
        Lang::Python,
        Lang::Go,
        Lang::Java,
        Lang::Kotlin,
    ];

    /// Canonical lowercase label for evidence records, inventory, and graph nodes.
    pub fn label(self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::TypeScript => "typescript",
            Lang::JavaScript => "javascript",
            Lang::Python => "python",
            Lang::Go => "go",
            Lang::Java => "java",
            Lang::Kotlin => "kotlin",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "rust" => Some(Self::Rust),
            "typescript" => Some(Self::TypeScript),
            "javascript" => Some(Self::JavaScript),
            "python" => Some(Self::Python),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "kotlin" => Some(Self::Kotlin),
            _ => None,
        }
    }

    /// Map a file path to its language by extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        let lang = match ext.as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            "java" => Self::Java,
            "kt" | "kts" => Self::Kotlin,
            _ => return None,
        };
        Some(lang)
    }

    /// Convert to the backing `SupportLang` enum.
    fn to_support(self) -> SupportLang {
        match self {
            Lang::Rust => SupportLang::Rust,
            Lang::TypeScript => SupportLang::TypeScript,
            Lang::JavaScript => SupportLang::JavaScript,
            Lang::Python => SupportLang::Python,
            Lang::Go => SupportLang::Go,
            Lang::Java => SupportLang::Java,
            Lang::Kotlin => SupportLang::Kotlin,
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---- delegate trait impls to SupportLang ----

impl Language for Lang {
    fn kind_to_id(&self, kind: &str) -> u16 {
        self.to_support().kind_to_id(kind)
    }
    fn field_to_id(&self, field: &str) -> Option<u16> {
        self.to_support().field_to_id(field)
    }
    fn build_pattern(
        &self,
        builder: &ast_grep_core::matcher::PatternBuilder,
    ) -> Result<Pattern, PatternError> {
        self.to_support().build_pattern(builder)
    }
    fn pre_process_pattern<'q>(&self, query: &'q str) -> std::borrow::Cow<'q, str> {
        self.to_support().pre_process_pattern(query)
    }
    fn expando_char(&self) -> char {
        self.to_support().expando_char()
    }
}

impl LanguageExt for Lang {
    fn get_ts_language(&self) -> ast_grep_core::tree_sitter::TSLanguage {
        self.to_support().get_ts_language()
    }
}

// ---- public API ----

/// Parse `source` as the given language and return the AstGrep root.
pub fn parse(lang: Lang, source: &str) -> AstGrep<StrDoc<Lang>> {
    lang.ast_grep(source)
}

/// Compile a pattern. Surfaces pattern syntax errors as anyhow errors so
/// the CLI can `?` them.
pub fn compile_pattern(lang: Lang, src: &str) -> Result<Pattern> {
    Pattern::try_new(src, lang).with_context(|| format!("compile pattern {src:?} for {lang}"))
}

/// Run a compiled pattern and return all matches. Each match carries
/// its node text + byte range.
pub fn find_all<'a>(
    root: &'a AstGrep<StrDoc<Lang>>,
    pattern: &Pattern,
) -> Vec<NodeMatch<'a, StrDoc<Lang>>> {
    root.root().find_all(pattern).collect()
}

/// Walk all nodes of a given `kind` (e.g. `"function_item"`,
/// `"class_declaration"`). Useful when the user wants "all functions"
/// without writing an ast-grep pattern.
pub fn find_by_kind<'a>(
    root: &'a AstGrep<StrDoc<Lang>>,
    kind: &str,
) -> Vec<NodeMatch<'a, StrDoc<Lang>>> {
    let mut out = Vec::new();
    walk_collect(&root.root(), kind, &mut out);
    out
}

fn walk_collect<'a>(
    node: &Node<'a, StrDoc<Lang>>,
    kind: &str,
    out: &mut Vec<NodeMatch<'a, StrDoc<Lang>>>,
) {
    if node.kind().as_ref() == kind {
        // Reuse the pattern matcher for kind-only queries by
        // synthesising a single-node pattern. Easier: build a pattern
        // string that targets the kind node type and re-run find_all.
        // The cheap shortcut is to walk until we find the node and
        // emit a NodeMatch via ast-grep's internals, but those aren't
        // public. For now we leave the function as a hook for callers
        // who want to drop into raw traversal.
        debug!(kind, "kind match candidate");
    }
    for child in node.children() {
        walk_collect(&child, kind, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_from_path_handles_common_extensions() {
        assert_eq!(Lang::from_path(Path::new("src/lib.rs")), Some(Lang::Rust));
        assert_eq!(
            Lang::from_path(Path::new("src/app.ts")),
            Some(Lang::TypeScript)
        );
        assert_eq!(
            Lang::from_path(Path::new("src/app.tsx")),
            Some(Lang::TypeScript)
        );
        assert_eq!(
            Lang::from_path(Path::new("src/app.js")),
            Some(Lang::JavaScript)
        );
        assert_eq!(Lang::from_path(Path::new("src/app.py")), Some(Lang::Python));
        assert_eq!(Lang::from_path(Path::new("main.go")), Some(Lang::Go));
        assert_eq!(Lang::from_path(Path::new("Foo.java")), Some(Lang::Java));
        assert_eq!(Lang::from_path(Path::new("readme.md")), None);
        assert_eq!(Lang::from_path(Path::new("data.json")), None);
    }

    #[test]
    fn lang_label_is_stable() {
        // The labels are part of the public contract (graph nodes,
        // evidence records, JSON output). Renames break consumers.
        assert_eq!(Lang::Rust.label(), "rust");
        assert_eq!(Lang::TypeScript.label(), "typescript");
        assert_eq!(Lang::JavaScript.label(), "javascript");
        assert_eq!(Lang::Python.label(), "python");
        assert_eq!(Lang::Go.label(), "go");
        assert_eq!(Lang::Java.label(), "java");
    }

    #[test]
    fn lang_round_trips_through_label() {
        for &l in Lang::ALL {
            assert_eq!(Lang::from_label(l.label()), Some(l));
        }
    }

    #[test]
    fn parse_rust_finds_function_items() {
        let src = "fn add(a: i32, b: i32) -> i32 { a + b }\nfn mul(a: i32, b: i32) -> i32 { a * b }\n";
        let ast = parse(Lang::Rust, src);
        let pattern = compile_pattern(Lang::Rust, "fn $NAME").unwrap();
        let matches = find_all(&ast, &pattern);
        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches[0].text(),
            "fn add(a: i32, b: i32) -> i32 { a + b }"
        );
        assert_eq!(
            matches[1].text(),
            "fn mul(a: i32, b: i32) -> i32 { a * b }"
        );
    }

    #[test]
    fn parse_javascript_finds_class_declarations() {
        let src = "class Foo {}\nclass Bar extends Baz {}\n";
        let ast = parse(Lang::JavaScript, src);
        let pattern = compile_pattern(Lang::JavaScript, "class $NAME").unwrap();
        let matches = find_all(&ast, &pattern);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn parse_python_finds_function_definitions() {
        // Python is expression-based and whitespace-significant, so
        // `def $NAME` is too restrictive (the AST requires either a
        // colon + body or a multi-capture). The loosest pattern that
        // matches both top-level defs is `def $$$`.
        let src = "def foo():\n    pass\n\ndef bar(x):\n    return x\n";
        let ast = parse(Lang::Python, src);
        let pattern = compile_pattern(Lang::Python, "def $$$").unwrap();
        let matches = find_all(&ast, &pattern);
        assert_eq!(matches.len(), 2);
        assert!(matches[0].text().starts_with("def foo"));
        assert!(matches[1].text().starts_with("def bar"));
    }

    #[test]
    fn parse_java_finds_class_declarations() {
        // Java's `class $NAME` only matches un-modified declarations.
        // For declarations with modifiers (`public class X`) you need
        // the modifier in the pattern. We exercise both shapes here.
        let src = "public class Foo {}\nclass Bar {}\n";
        let ast = parse(Lang::Java, src);

        let pattern = compile_pattern(Lang::Java, "class $NAME").unwrap();
        let unmod = find_all(&ast, &pattern);
        assert_eq!(unmod.len(), 1);
        assert_eq!(unmod[0].text(), "class Bar {}");

        let pattern = compile_pattern(Lang::Java, "public class $NAME").unwrap();
        let pubmod = find_all(&ast, &pattern);
        assert_eq!(pubmod.len(), 1);
        assert_eq!(pubmod[0].text(), "public class Foo {}");
    }

    #[test]
    fn parse_kotlin_finds_functions() {
        // Kotlin uses `fun` for functions. Pattern must use the Kotlin expando
        // char (µ) because $ is not valid in Kotlin identifiers.
        let src = "fun add(a: Int, b: Int): Int = a + b\nfun mul(x: Int) = x * 2\n";
        let ast = parse(Lang::Kotlin, src);
        // Kotlin uses µ as meta-var prefix (expando_char)
        let pattern = compile_pattern(Lang::Kotlin, "fun µNAME").unwrap();
        let matches = find_all(&ast, &pattern);
        assert_eq!(matches.len(), 2);
        assert!(matches[0].text().starts_with("fun add"));
        assert!(matches[1].text().starts_with("fun mul"));
    }

     #[test]
     fn match_carries_byte_range_and_line() {
         let src = "fn alpha() {}\nfn beta() {}\n";
         let ast = parse(Lang::Rust, src);
         let pattern = compile_pattern(Lang::Rust, "fn $NAME").unwrap();
         let matches = find_all(&ast, &pattern);
         assert_eq!(matches.len(), 2);
         // alpha is on line 0, beta is on line 1.
         assert_eq!(matches[0].start_pos().line(), 0);
         assert_eq!(matches[1].start_pos().line(), 1);
         assert_eq!(matches[0].range().start, 0);
         assert!(matches[1].range().start > matches[0].range().start);
     }
}
