//! The manifest `aye.toml`: a strict subset of TOML (`docs/decisions.md`,
//! "Dependencies (O4)").
//!
//! Comment lines (`#`), blank lines, `name = "app"`, then a `[deps]` table
//! with one line per dependency, `key = { path = "dir" }` or
//! `key = { url = "https://.../x.tar.gz" }`. A string has no escapes. Every
//! file accepted here is valid TOML with the same meaning; anything else is
//! E0119.

use crate::codes::*;
use crate::diag::{Error, Span};
use crate::module::SourceFile;
use std::path::{Path, PathBuf};

/// The file name of a manifest.
pub const FILE: &str = "aye.toml";

/// What `[deps]` says about one dependency.
#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    /// A directory, relative to the manifest or absolute.
    Path(String),
    /// An `https://` or `file://` URL of a `.tar.gz` or `.tgz` archive.
    Url(String),
}

#[derive(Debug)]
pub struct Dep {
    pub key: String,
    pub source: Source,
    /// Where the key is (spans have `file: 0`; the loader sets the file).
    pub span: Span,
}

#[derive(Debug)]
pub struct Manifest {
    pub name: String,
    pub deps: Vec<Dep>,
}

/// A manifest file and the directory it is in (a dependency's `path` is
/// relative to it).
pub struct ManifestFile {
    pub file: SourceFile,
    pub dir: PathBuf,
}

impl ManifestFile {
    /// The `aye.toml` in `dir`, if there is one.
    pub fn read(dir: &Path) -> Result<Option<ManifestFile>, String> {
        let path = dir.join(FILE);
        if !path.is_file() {
            return Ok(None);
        }
        let name = path.to_string_lossy().into_owned();
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("cannot read {name}: {e}"))?;
        Ok(Some(ManifestFile {
            file: SourceFile {
                name,
                text,
                std: false,
                dep: true,
            },
            dir: dir.to_path_buf(),
        }))
    }

    /// The `name` of the manifest, if it parses.
    pub fn name(&self) -> Option<String> {
        parse(&self.file.text).ok().map(|m| m.name)
    }

    /// The `aye.toml` in `start` or the nearest directory above it, up to
    /// and including `top`.
    pub fn nearest(start: &Path, top: &Path) -> Result<Option<ManifestFile>, String> {
        let canon = |p: &Path| {
            let p = if p.as_os_str().is_empty() {
                Path::new(".")
            } else {
                p
            };
            std::fs::canonicalize(p).unwrap_or(p.to_path_buf())
        };
        let top = canon(top);
        let mut dir = start.to_path_buf();
        loop {
            if let Some(m) = ManifestFile::read(&dir)? {
                return Ok(Some(m));
            }
            if canon(&dir) == top {
                return Ok(None);
            }
            match dir.parent() {
                Some(p) => dir = p.to_path_buf(),
                None => return Ok(None),
            }
        }
    }
}

const SHAPE: &str = "expected `name = \"app\"`, then `[deps]` with lines such as `util = { path = \"../util\" }` or `lib = { url = \"https://example.com/lib.tar.gz\" }`";

/// Is `s` an identifier (`[A-Za-z_][A-Za-z0-9_]*`)?
pub fn is_ident(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// One line, read from left to right.
struct Cursor<'a> {
    s: &'a str,
    pos: usize,
    line: u32,
}

impl Cursor<'_> {
    fn span(&self) -> Span {
        Span::new(self.line, self.s[..self.pos].chars().count() as u32 + 1)
    }

    fn err(&self, what: &str) -> Error {
        Error::new(E0119, self.span(), format!("{what}; {SHAPE}"))
    }

    fn rest(&self) -> &str {
        &self.s[self.pos..]
    }

    fn skip_ws(&mut self) {
        self.pos = self.s.len() - self.rest().trim_start().len();
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip_ws();
        if self.rest().starts_with(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    /// A bare key: letters, digits, `_` and `-`.
    fn key(&mut self) -> Option<String> {
        self.skip_ws();
        let n = self
            .rest()
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(self.rest().len());
        if n == 0 {
            return None;
        }
        let k = self.rest()[..n].to_string();
        self.pos += n;
        Some(k)
    }

    /// A string with no escapes.
    fn string(&mut self) -> Result<String, Error> {
        self.skip_ws();
        if !self.rest().starts_with('"') {
            return Err(self.err("expected a string in double quotes"));
        }
        let body = &self.rest()[1..];
        let Some(n) = body.find('"') else {
            return Err(self.err("the string is not closed"));
        };
        if let Some(b) = body[..n].find('\\') {
            self.pos += 1 + b;
            return Err(self.err("a string has no escapes (a `\\` is not accepted)"));
        }
        let v = body[..n].to_string();
        self.pos += n + 2;
        Ok(v)
    }

    /// The line has nothing more.
    fn end(&mut self) -> Result<(), Error> {
        self.skip_ws();
        if self.rest().is_empty() {
            Ok(())
        } else {
            Err(self.err("unexpected text after the value"))
        }
    }
}

/// `{ path = "..." }` or `{ url = "..." }`.
fn source(c: &mut Cursor) -> Result<Source, Error> {
    if !c.eat('{') {
        return Err(c.err("a dependency is an inline table"));
    }
    let mut found: Vec<(String, String, Span)> = Vec::new();
    loop {
        if c.eat('}') {
            break;
        }
        c.skip_ws();
        let at = c.span();
        let Some(field) = c.key() else {
            return Err(c.err("expected `path` or `url`"));
        };
        if field != "path" && field != "url" {
            return Err(Error::new(
                E0119,
                at,
                format!("unknown field `{field}` of a dependency; {SHAPE}"),
            ));
        }
        if !c.eat('=') {
            return Err(c.err("expected `=`"));
        }
        let value = c.string()?;
        if found.iter().any(|(f, ..)| *f == field) {
            return Err(Error::new(E0119, at, format!("`{field}` is given twice")));
        }
        found.push((field, value, at));
        if !c.eat(',') {
            if !c.eat('}') {
                return Err(c.err("expected `,` or `}`"));
            }
            break;
        }
    }
    let [(field, value, at)] = &found[..] else {
        return Err(c.err("a dependency has exactly one of `path` and `url`"));
    };
    if value.is_empty() {
        return Err(Error::new(E0119, *at, format!("`{field}` is empty")));
    }
    if field == "path" {
        return Ok(Source::Path(value.clone()));
    }
    let scheme = value.starts_with("https://") || value.starts_with("file://");
    if !scheme || !(value.ends_with(".tar.gz") || value.ends_with(".tgz")) {
        let msg = "a `url` is an `https://` or `file://` URL of a `.tar.gz` or `.tgz` archive";
        return Err(Error::new(E0119, *at, msg));
    }
    Ok(Source::Url(value.clone()))
}

/// Read a manifest. The errors have `file: 0`.
pub fn parse(text: &str) -> Result<Manifest, Vec<Error>> {
    let mut errors = Vec::new();
    let mut name: Option<String> = None;
    let mut deps: Vec<Dep> = Vec::new();
    let mut in_deps = false;
    let mut seen_deps = false;
    for (i, raw) in text.lines().enumerate() {
        let line = i as u32 + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let mut c = Cursor {
            s: raw,
            pos: 0,
            line,
        };
        let r = (|| -> Result<(), Error> {
            if c.eat('[') {
                c.skip_ws();
                let at = c.span();
                let table = c.key();
                if table.as_deref() != Some("deps") || !c.eat(']') {
                    return Err(Error::new(
                        E0119,
                        at,
                        format!("the only table is `[deps]`; {SHAPE}"),
                    ));
                }
                if seen_deps {
                    return Err(Error::new(E0119, at, "`[deps]` is given twice"));
                }
                seen_deps = true;
                in_deps = true;
                return c.end();
            }
            c.skip_ws();
            let at = c.span();
            let Some(key) = c.key() else {
                return Err(c.err("expected a key"));
            };
            if !c.eat('=') {
                return Err(
                    c.err("expected `=` after the key (a dotted or quoted key is not accepted)")
                );
            }
            if in_deps {
                if deps.iter().any(|d| d.key == key) {
                    return Err(Error::new(E0119, at, format!("`{key}` is given twice")));
                }
                let source = source(&mut c)?;
                c.end()?;
                deps.push(Dep {
                    key,
                    source,
                    span: at,
                });
                return Ok(());
            }
            if key != "name" {
                return Err(Error::new(
                    E0119,
                    at,
                    format!("unknown field `{key}`; {SHAPE}"),
                ));
            }
            if name.is_some() {
                return Err(Error::new(E0119, at, "`name` is given twice"));
            }
            let at = {
                c.skip_ws();
                c.span()
            };
            let v = c.string()?;
            c.end()?;
            if !is_ident(&v) || crate::lexer::is_keyword(&v) {
                return Err(Error::new(
                    E0119,
                    at,
                    format!("`name` must be an identifier, not \"{v}\""),
                ));
            }
            name = Some(v);
            Ok(())
        })();
        if let Err(e) = r {
            errors.push(e);
        }
    }
    if name.is_none() && errors.is_empty() {
        errors.push(Error::new(
            E0119,
            Span::new(1, 1),
            format!("`name` is missing; {SHAPE}"),
        ));
    }
    match name {
        Some(name) if errors.is_empty() => Ok(Manifest { name, deps }),
        _ => Err(errors),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(text: &str) -> (u32, u32, String) {
        let e = &parse(text).unwrap_err()[0];
        (e.span.line, e.span.col, e.msg.clone())
    }

    #[test]
    fn reads_a_manifest() {
        let m = parse(
            "# app\nname = \"app\"\n\n[deps]\nutil = { path = \"../util\" }\nlib = { url = \"https://x.org/lib.tgz\" }\n",
        )
        .unwrap();
        assert_eq!(m.name, "app");
        assert_eq!(m.deps.len(), 2);
        assert_eq!(m.deps[0].source, Source::Path("../util".into()));
        assert_eq!(m.deps[0].span, Span::new(5, 1));
        assert_eq!(
            m.deps[1].source,
            Source::Url("https://x.org/lib.tgz".into())
        );
    }

    #[test]
    fn rejects_what_the_subset_lacks() {
        assert_eq!(err("name = \"a\"\n[deps]\nu = { path = \"x\\y\" }\n").0, 3);
        assert_eq!(err("name = \"a\"\n[deps.u]\n").0, 2);
        assert_eq!(err("name = \"a\"\n[deps]\na.b = { path = \"x\" }\n").0, 3);
        assert_eq!(
            err("name = \"a\"\n[deps]\nu = { path = \"x\", url = \"https://h/a.tgz\" }\n").0,
            3
        );
        assert_eq!(err("name = \"a\"\n[deps]\nu = { }\n").0, 3);
        assert_eq!(err("name = \"a\"\n[deps]\nu = { git = \"x\" }\n").0, 3);
        assert_eq!(
            err("name = \"a\"\n[deps]\nu = { url = \"http://h/a.tgz\" }\n").0,
            3
        );
        assert_eq!(err("name = \"a\"\nversion = 1\n").0, 2);
        assert_eq!(err("[deps]\n").0, 1);
        assert!(err("name = \"a b\"\n").2.contains("identifier"));
    }
}
