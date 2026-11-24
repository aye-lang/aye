//! Bounds-check elimination on OIR: leaves out the index, slice and `+ 1` /
//! `- 1` overflow checks that facts known at that point prove.
//!
//! A fact is about locals that only an assignment in the function can
//! change: not globals, and not locals whose storage has its address taken
//! (`&x`, `&x.f`, `&mut x` for a method, a slice of an array in `x`), since
//! a call or a store through a pointer could change those. Facts come from
//! the comparisons that guard a branch (`i < s.len`, `i >= n`, `0 < k`,
//! through `&&`, `||` and `!`), from the checks already passed (`s[i]`
//! proves the next `s[i]`), and from copies (`let n = s.len`, the copy of
//! the sequence a `for x in xs` makes). A fact holds until one of its
//! locals is assigned; a loop starts with the facts none of whose locals it
//! assigns, so no fixpoint is needed. A branch joins the facts of the paths
//! that reach its end.
//!
//! Checks inside `@setSafety(false)` add no facts (they do not run), so a
//! proof never rests on a check that was left out. What panics does not
//! change: a check is left out only where it cannot fail.

use crate::ast::BinOp;
use crate::oir::*;
use crate::sema::{IntTy, Type};
use std::collections::HashSet;

/// At most this many facts are kept at a point (the oldest go first), so a
/// long function costs linear time.
const MAX_FACTS: usize = 128;

/// The checks a run left out.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Removed {
    pub index: usize,
    pub slice: usize,
    pub overflow: usize,
}

impl Removed {
    fn add(&mut self, o: Removed) {
        self.index += o.index;
        self.slice += o.slice;
        self.overflow += o.overflow;
    }
}

/// Run the pass over every function of the program.
pub fn program(p: &mut Program) -> Removed {
    let mut r = Removed::default();
    for f in &mut p.funcs {
        r.add(func(f));
    }
    r
}

/// A value stored in a local: the local itself, or a field of it (`path`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    local: LocalId,
    path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Term {
    /// The value at the key.
    Val(Key),
    /// The `len` of the slice at the key.
    Len(Key),
    /// An integer (or `bool` as 0 and 1).
    Const(i128),
}

impl Term {
    fn mentions(&self, l: LocalId) -> bool {
        match self {
            Term::Val(k) | Term::Len(k) => k.local == l,
            Term::Const(_) => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Fact {
    /// `a < b`, compared as the integer type (the index check: `usize`).
    Lt(Term, Term, IntTy),
    /// The local holds the term's value.
    Eq(LocalId, Term),
    /// When the `bool` local is the given value, `a < b` holds.
    Cond(LocalId, bool, Term, Term, IntTy),
}

impl Fact {
    fn mentions(&self, l: LocalId) -> bool {
        match self {
            Fact::Lt(a, b, _) => a.mentions(l) || b.mentions(l),
            Fact::Eq(x, t) => *x == l || t.mentions(l),
            Fact::Cond(c, _, a, b, _) => *c == l || a.mentions(l) || b.mentions(l),
        }
    }
}

/// The facts at a point; `dead` when no path reaches it.
#[derive(Clone, Debug, Default)]
struct Facts {
    v: Vec<Fact>,
    dead: bool,
}

impl Facts {
    fn add(&mut self, f: Fact) {
        if self.dead || self.v.contains(&f) {
            return;
        }
        if self.v.len() == MAX_FACTS {
            self.v.remove(0);
        }
        self.v.push(f);
    }

    fn kill(&mut self, l: LocalId) {
        self.v.retain(|f| !f.mentions(l));
    }

    fn die(&mut self) {
        self.v.clear();
        self.dead = true;
    }

    fn has(&self, f: &Fact) -> bool {
        self.v.contains(f)
    }

    /// Is `a < b` known (in any type)?
    fn lt(&self, a: &Term, b: &Term) -> bool {
        self.v
            .iter()
            .any(|f| matches!(f, Fact::Lt(x, y, _) if x == a && y == b))
    }

    /// Is `a < m` known for a constant `m <= n`?
    fn lt_const(&self, a: &Term, n: i128) -> bool {
        self.v
            .iter()
            .any(|f| matches!(f, Fact::Lt(x, Term::Const(m), _) if x == a && *m <= n))
    }

    /// Is `a < x` (`up`) or `x < a` (not `up`) known in the type `ty`, for
    /// some `x`? Then `a + 1` (`a - 1`) does not overflow `ty`.
    fn bounded(&self, a: &Term, ty: IntTy, up: bool) -> bool {
        self.v.iter().any(|f| match f {
            Fact::Lt(x, y, t) if *t == ty => (if up { x } else { y }) == a,
            _ => false,
        })
    }
}

/// The facts that hold after a branch whose two ends are `a` and `b`;
/// `bools` are the `bool` locals.
fn join(a: Facts, b: Facts, bools: &[bool]) -> Facts {
    if a.dead {
        return b;
    }
    if b.dead {
        return a;
    }
    let mut out = Facts::default();
    let side = |x: &Facts, y: &Facts, out: &mut Facts| {
        for f in &x.v {
            match f {
                _ if y.has(f) => out.add(f.clone()),
                // `c` implies `a < b` on one side; on the other `a < b`
                // holds anyway, or `c` has the other value.
                Fact::Cond(c, w, p, q, t) => {
                    let lt = Fact::Lt(p.clone(), q.clone(), *t);
                    if y.has(&lt) || y.has(&Fact::Eq(*c, Term::Const(!*w as i128))) {
                        out.add(f.clone());
                    }
                }
                // `a < b` on one side, `c` false (true) on the other: `c`
                // true (false) implies `a < b`.
                Fact::Lt(p, q, t) => {
                    for g in &y.v {
                        if let Fact::Eq(c, Term::Const(v @ (0 | 1))) = g {
                            if !bools[*c] {
                                continue;
                            }
                            out.add(Fact::Cond(*c, *v == 0, p.clone(), q.clone(), *t));
                        }
                    }
                }
                _ => {}
            }
        }
    };
    side(&a, &b, &mut out);
    side(&b, &a, &mut out);
    out
}

/// Does assigning `p` write the storage of `p.local` (not memory that a
/// pointer or a slice in it points to)?
fn in_storage(p: &Place) -> bool {
    !p.proj.iter().any(|j| {
        matches!(
            j,
            Proj::Deref
                | Proj::Index {
                    len: Len::Slice | Len::Unchecked,
                    ..
                }
        )
    })
}

/// The locals whose storage `b` assigns.
fn assigned(b: &Block, out: &mut HashSet<LocalId>) {
    for s in b {
        match &s.kind {
            StmtKind::Assign(p, _) if in_storage(p) => {
                out.insert(p.local);
            }
            StmtKind::If { then, els, .. } => {
                assigned(then, out);
                assigned(els, out);
            }
            StmtKind::Loop { body, cont } => {
                assigned(body, out);
                assigned(cont, out);
            }
            StmtKind::Safety { body, .. } => assigned(body, out),
            _ => {}
        }
    }
}

/// Mark the locals whose storage `b` takes the address of.
fn escaped(b: &Block, out: &mut [bool]) {
    let rv = |rv: &Rvalue, out: &mut [bool]| match rv {
        Rvalue::AddrOf(p) if in_storage(p) => out[p.local] = true,
        Rvalue::Slice {
            base,
            len: Len::Array(_) | Len::Lanes(_),
            ..
        } if in_storage(base) => out[base.local] = true,
        _ => {}
    };
    for s in b {
        match &s.kind {
            StmtKind::Assign(_, r) | StmtKind::Eval(r) => rv(r, out),
            StmtKind::If { then, els, .. } => {
                escaped(then, out);
                escaped(els, out);
            }
            StmtKind::Loop { body, cont } => {
                escaped(body, out);
                escaped(cont, out);
            }
            StmtKind::Safety { body, .. } => escaped(body, out),
            _ => {}
        }
    }
}

/// Run the pass over one function.
pub fn func(f: &mut Func) -> Removed {
    let mut bad: Vec<bool> = f.locals.iter().map(|l| l.global).collect();
    escaped(&f.body, &mut bad);
    let mut p = Pass {
        locals: &f.locals,
        bad,
        bools: f.locals.iter().map(|l| l.ty == Type::Bool).collect(),
        checking: true,
        removed: Removed::default(),
    };
    let mut facts = Facts::default();
    p.block(&mut f.body, &mut facts);
    p.removed
}

struct Pass<'a> {
    locals: &'a [Local],
    /// Locals that facts may not mention (globals, escaped storage).
    bad: Vec<bool>,
    /// The `bool` locals.
    bools: Vec<bool>,
    /// The checks of the current block run (not `@setSafety(false)`).
    checking: bool,
    removed: Removed,
}

impl Pass<'_> {
    /// `k` with a copied local replaced by what it copies.
    fn norm_key(&self, f: &Facts, k: Key) -> Key {
        for x in &f.v {
            if let Fact::Eq(l, Term::Val(k2)) = x {
                if *l == k.local {
                    let mut path = k2.path.clone();
                    path.extend(k.path);
                    return Key {
                        local: k2.local,
                        path,
                    };
                }
            }
        }
        k
    }

    fn norm(&self, f: &Facts, t: Term) -> Term {
        match t {
            Term::Val(k) if k.path.is_empty() => {
                for x in &f.v {
                    if let Fact::Eq(l, t2) = x {
                        if *l == k.local {
                            return t2.clone();
                        }
                    }
                }
                Term::Val(k)
            }
            Term::Val(k) => Term::Val(self.norm_key(f, k)),
            Term::Len(k) => Term::Len(self.norm_key(f, k)),
            c => c,
        }
    }

    fn operand(&self, f: &Facts, op: &Operand) -> Option<Term> {
        match op {
            Operand::Local(l) if !self.bad[*l] => Some(self.norm(
                f,
                Term::Val(Key {
                    local: *l,
                    path: vec![],
                }),
            )),
            Operand::Int(v, _) => Some(Term::Const(*v)),
            Operand::Bool(b) => Some(Term::Const(*b as i128)),
            _ => None,
        }
    }

    /// The key of `local` followed by `proj`, when `proj` is only fields.
    fn key(&self, f: &Facts, local: LocalId, proj: &[Proj]) -> Option<Key> {
        if self.bad[local] {
            return None;
        }
        let mut path = Vec::new();
        for j in proj {
            match j {
                Proj::Field(n) => path.push(n.clone()),
                _ => return None,
            }
        }
        Some(self.norm_key(f, Key { local, path }))
    }

    /// The value a load of `p` reads, as a term.
    fn load(&self, f: &Facts, p: &Place) -> Option<Term> {
        match p.proj.split_last() {
            None => self.operand(f, &Operand::Local(p.local)),
            Some((Proj::Field(n), rest)) if n == "len" => {
                Some(Term::Len(self.key(f, p.local, rest)?))
            }
            Some(_) => Some(Term::Val(self.key(f, p.local, &p.proj)?)),
        }
    }

    /// `a <= b`?
    fn le(&self, f: &Facts, a: &Term, b: &Term) -> bool {
        match (a, b) {
            _ if a == b => true,
            // Bounds are unsigned.
            (Term::Const(0), _) => true,
            (Term::Const(x), Term::Const(y)) => x <= y,
            (_, Term::Const(n)) => f.lt_const(a, *n),
            _ => f.lt(a, b),
        }
    }

    /// The facts that hold when the `bool` local `c` is `when`.
    fn assume(&self, f: &mut Facts, c: LocalId, when: bool) {
        let new: Vec<Fact> =
            f.v.iter()
                .filter_map(|x| match x {
                    Fact::Cond(l, w, a, b, t) if *l == c && *w == when => {
                        Some(Fact::Lt(a.clone(), b.clone(), *t))
                    }
                    _ => None,
                })
                .collect();
        for x in new {
            f.add(x);
        }
    }

    /// The index checks of `p`, in the order the C backend emits them.
    fn place(&mut self, p: &mut Place, f: &mut Facts) {
        for k in 0..p.proj.len() {
            let base = self.key(f, p.local, &p.proj[..k]);
            let Proj::Index {
                index,
                len,
                checked,
                ..
            } = &mut p.proj[k]
            else {
                continue;
            };
            if !*checked {
                continue;
            }
            let Some(i) = self.operand(f, index) else {
                continue;
            };
            let bound = match len {
                Len::Slice => match base {
                    Some(b) => Term::Len(b),
                    None => continue,
                },
                Len::Array(n) | Len::Lanes(n) => {
                    // The C backend leaves out a constant index's check.
                    if matches!(index, Operand::Int(..)) {
                        continue;
                    }
                    Term::Const(*n as i128)
                }
                Len::Unchecked => continue,
            };
            let proved = match &bound {
                Term::Const(n) => f.lt_const(&i, *n),
                b => f.lt(&i, b),
            };
            if proved {
                *checked = false;
                self.removed.index += 1;
            } else if self.checking {
                f.add(Fact::Lt(i, bound, IntTy::Usize));
            }
        }
    }

    fn rvalue(&mut self, rv: &mut Rvalue, f: &mut Facts) {
        match rv {
            Rvalue::Load(p)
            | Rvalue::AddrOf(p)
            | Rvalue::IsSome { place: p, .. }
            | Rvalue::IsErr(p) => self.place(p, f),
            Rvalue::Slice {
                base,
                len,
                lo,
                hi,
                checked,
                ..
            } => {
                self.place(base, f);
                let n = match len {
                    Len::Slice => match self.key(f, base.local, &base.proj) {
                        Some(k) => Term::Len(k),
                        None => return,
                    },
                    Len::Array(n) | Len::Lanes(n) => Term::Const(*n as i128),
                    Len::Unchecked => return,
                };
                let Some(lo) = self.operand(f, lo) else {
                    return;
                };
                let hi = match hi {
                    Some(h) => match self.operand(f, h) {
                        Some(t) => t,
                        None => return,
                    },
                    None => n.clone(),
                };
                if *checked && self.le(f, &lo, &hi) && self.le(f, &hi, &n) {
                    *checked = false;
                    self.removed.slice += 1;
                }
            }
            _ => {}
        }
    }

    /// `i + 1` with `i < x` known, or `i - 1` with `x < i`, cannot
    /// overflow: make it wrapping (no check).
    fn overflow(&mut self, rv: &mut Rvalue, f: &Facts) {
        let Rvalue::Binary {
            op,
            lhs,
            rhs,
            ty: Type::Int(it),
            ..
        } = rv
        else {
            return;
        };
        let one = |o: &Operand| matches!(o, Operand::Int(1, _));
        let up = match op {
            BinOp::Add if one(rhs) => Some((lhs, true)),
            BinOp::Add if one(lhs) => Some((rhs, true)),
            BinOp::Sub if one(rhs) => Some((lhs, false)),
            _ => None,
        };
        let Some((x, up)) = up else {
            return;
        };
        let Some(t) = self.operand(f, x) else {
            return;
        };
        if f.bounded(&t, *it, up) {
            *op = if up { BinOp::AddWrap } else { BinOp::SubWrap };
            self.removed.overflow += 1;
        }
    }

    /// The facts that `x = rv` makes, from the facts before it.
    fn derive(&self, x: LocalId, rv: &Rvalue, f: &Facts) -> Vec<Fact> {
        if self.bad[x] {
            return vec![];
        }
        let ty = &self.locals[x].ty;
        let value = matches!(ty, Type::Int(_) | Type::Slice(..) | Type::Bool);
        let eq = |t: Option<Term>| match t {
            Some(t) if value && !t.mentions(x) => vec![Fact::Eq(x, t)],
            _ => vec![],
        };
        match rv {
            Rvalue::Use(op @ (Operand::Int(..) | Operand::Bool(_))) => eq(self.operand(f, op)),
            Rvalue::Use(Operand::Local(y)) if *ty == Type::Bool => {
                // A copy of a condition implies what it does.
                let mut out = eq(self.operand(f, &Operand::Local(*y)));
                for c in &f.v {
                    if let Fact::Cond(l, w, a, b, t) = c {
                        if l == y && !a.mentions(x) && !b.mentions(x) {
                            out.push(Fact::Cond(x, *w, a.clone(), b.clone(), *t));
                        }
                    }
                }
                out
            }
            Rvalue::Use(op) => eq(self.operand(f, op)),
            Rvalue::Load(p) => eq(self.load(f, p)),
            Rvalue::Not(Operand::Local(y)) => {
                f.v.iter()
                    .filter_map(|c| match c {
                        Fact::Cond(l, w, a, b, t) if l == y && !a.mentions(x) && !b.mentions(x) => {
                            Some(Fact::Cond(x, !*w, a.clone(), b.clone(), *t))
                        }
                        _ => None,
                    })
                    .collect()
            }
            Rvalue::Binary {
                op: op @ (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge),
                lhs,
                rhs,
                ty: Type::Int(it),
                ..
            } => {
                let (Some(a), Some(b)) = (self.operand(f, lhs), self.operand(f, rhs)) else {
                    return vec![];
                };
                if a.mentions(x) || b.mentions(x) {
                    return vec![];
                }
                // `a < b` when true; `a >= b` when false; `a > b` is `b < a`
                // when true; `a <= b` is `b < a` when false.
                let (when, a, b) = match op {
                    BinOp::Lt => (true, a, b),
                    BinOp::Ge => (false, a, b),
                    BinOp::Gt => (true, b, a),
                    _ => (false, b, a),
                };
                vec![Fact::Cond(x, when, a, b, *it)]
            }
            _ => vec![],
        }
    }

    fn block(&mut self, b: &mut Block, f: &mut Facts) {
        for s in b {
            self.stmt(s, f);
        }
    }

    fn stmt(&mut self, s: &mut Stmt, f: &mut Facts) {
        match &mut s.kind {
            StmtKind::Assign(place, rv) => {
                self.place(place, f);
                self.rvalue(rv, f);
                self.overflow(rv, f);
                let new = if place.proj.is_empty() {
                    self.derive(place.local, rv, f)
                } else {
                    vec![]
                };
                if in_storage(place) {
                    f.kill(place.local);
                }
                for x in new {
                    f.add(x);
                }
            }
            StmtKind::Eval(rv) => self.rvalue(rv, f),
            StmtKind::If { cond, then, els } => {
                let mut a = f.clone();
                let mut b = f.clone();
                if let Operand::Local(c) = cond {
                    self.assume(&mut a, *c, true);
                    self.assume(&mut b, *c, false);
                }
                self.block(then, &mut a);
                self.block(els, &mut b);
                *f = join(a, b, &self.bools);
            }
            StmtKind::Loop { body, cont } => {
                // The facts of the loop's entry that hold on every
                // iteration: the loop assigns none of their locals.
                let mut changed = HashSet::new();
                assigned(body, &mut changed);
                assigned(cont, &mut changed);
                let mut head = f.clone();
                head.dead = false;
                head.v.retain(|x| !changed.iter().any(|l| x.mentions(*l)));
                self.block(body, &mut head.clone());
                self.block(cont, &mut head.clone());
                *f = head;
            }
            StmtKind::Break | StmtKind::Continue | StmtKind::Return(_) => f.die(),
            StmtKind::Check {
                cond: Some(Operand::Local(c)),
                ..
            } => {
                if self.checking {
                    let c = *c;
                    self.assume(f, c, true);
                }
            }
            StmtKind::Check { cond: None, .. } => f.die(),
            StmtKind::Check { .. } | StmtKind::TracePush(_) | StmtKind::TraceRestore(_) => {}
            StmtKind::Safety { on, body } => {
                let saved = self.checking;
                self.checking = *on;
                self.block(body, f);
                self.checking = saved;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the pass leaves out of function `name` in `src`, and how many
    /// index checks with a non-constant index it keeps.
    fn run(src: &str, name: &str) -> (Removed, usize) {
        let typed = crate::driver::check("t.aye", src).expect("compiles");
        let ir = crate::oir::lower_checked(&typed, &[], 0);
        let mut f = ir.funcs.into_iter().find(|f| f.name == name).unwrap();
        let removed = func(&mut f);
        fn kept(b: &Block) -> usize {
            let place = |p: &Place| {
                p.proj
                    .iter()
                    .filter(|j| {
                        matches!(
                            j,
                            Proj::Index {
                                index: Operand::Local(_),
                                len: Len::Slice | Len::Array(_),
                                checked: true,
                                ..
                            }
                        )
                    })
                    .count()
            };
            b.iter()
                .map(|s| match &s.kind {
                    StmtKind::Assign(p, rv) => {
                        place(p)
                            + match rv {
                                Rvalue::Load(q) | Rvalue::AddrOf(q) => place(q),
                                _ => 0,
                            }
                    }
                    StmtKind::If { then, els, .. } => kept(then) + kept(els),
                    StmtKind::Loop { body, cont } => kept(body) + kept(cont),
                    StmtKind::Safety { body, .. } => kept(body),
                    _ => 0,
                })
                .sum()
        }
        (removed, kept(&f.body))
    }

    fn removed(index: usize, slice: usize, overflow: usize) -> Removed {
        Removed {
            index,
            slice,
            overflow,
        }
    }

    /// `main` calls `f` (a function nothing calls is not lowered).
    fn main(args: &str) -> String {
        format!("pub fn main() {{\n    var a = [2]u32{{1, 2}}\n    _ = f(a[..]{args})\n}}\n")
    }

    #[test]
    fn loop_bounds_leave_out_index_and_step_checks() {
        let src = format!(
            "{}fn f(s: []u32) u32 {{
    var t: u32 = 0
    for i in 0..s.len {{
        t +%= s[i]
    }}
    var j: usize = 0
    while j < s.len {{
        t +%= s[j] +% s[j]
        j += 1
    }}
    for k, x in s {{
        t +%= x +% s[k]
    }}
    let n = s.len
    var m: usize = 0
    while m < n {{
        t +%= s[m]
        m += 1
    }}
    return t
}}
",
            main("")
        );
        // `s[i]`; `s[j]` twice; the element of `for` and `s[k]`; `s[m]`.
        // `j + 1` and `m + 1`.
        assert_eq!(run(&src, "f"), (removed(6, 0, 2), 0));
    }

    #[test]
    fn conditions_through_and_or_not() {
        let src = format!(
            "{}fn f(s: []u32, i: usize) u32 {{
    if i < s.len && s[i] > 0 {{
        return s[i]
    }}
    if i >= s.len || s[i] == 0 {{
        return 0
    }}
    if !(i >= s.len) {{
        return s[i]
    }}
    return 1
}}
",
            main(", 1")
        );
        assert_eq!(run(&src, "f"), (removed(4, 0, 0), 0));
    }

    #[test]
    fn slices_within_known_bounds() {
        let src = format!(
            "{}fn f(s: []u32, i: usize) usize {{
    if i < s.len {{
        return s[i..].len + s[..i].len + s[..].len
    }}
    return s[i..].len
}}
",
            main(", 1")
        );
        // `s[i..]` after the `if` is not known to be in bounds.
        assert_eq!(run(&src, "f").0, removed(0, 3, 0));
    }

    #[test]
    fn checks_that_stay() {
        let src = format!(
            "{}fn f(a: []u32, b: []u32) u32 {{
    var s = a
    var t: u32 = 0
    for i in 0..s.len {{
        s = b
        t +%= s[i]
    }}
    for i in 0..a.len {{
        t +%= b[i]
    }}
    var j: usize = 0
    while j <= a.len {{
        t +%= a[j]
        j += 1
    }}
    var k: usize = 0
    while k < a.len {{
        k += 1
        t +%= a[k]
    }}
    var c = a
    let q = &mut c
    var m: usize = 0
    while m < c.len {{
        q.* = b
        t +%= c[m]
        m += 1
    }}
    return t
}}
",
            main(", a[..]")
        );
        // The five indexes keep their checks. `j + 1` follows the check of
        // `a[j]`, `k + 1` follows `k < a.len`, `m + 1` follows `m < c.len`
        // (a value read before): they cannot overflow.
        assert_eq!(run(&src, "f"), (removed(0, 0, 3), 5));
    }
}
