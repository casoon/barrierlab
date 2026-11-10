//! Compiles the syntax tree to a Thompson NFA program and runs it as a Pike VM:
//! all threads advance over the input in lock step, in priority order, so a
//! search costs O(program x input) however the pattern is written, and the
//! machine never recurses on the input. Priority order gives the leftmost,
//! first-alternative-wins, greedy/reluctant semantics F&O 7.6 asks for.

use super::RegexError;
use super::class::Class;
use super::parse::{Node, Parsed};

/// Instructions a program may have; counted quantifiers are expanded, so this
/// bounds `(a{100}){100}`.
pub(super) const MAX_PROGRAM: usize = 10_000;

/// Thread steps per input character and program instruction one call (all
/// matches of a `replace`/`tokenize` together) may spend: a search costs at
/// most about two steps per instruction and character, so this is linear work
/// with room for matches that look ahead and for the start thread of every
/// position. Quadratic re-scanning (`a.*z|a` over many `a`) exceeds it.
const STEPS_PER_CELL: u64 = 8;
/// Whatever the product, no call spends more than this (a few seconds).
const STEP_CEILING: u64 = 2_000_000_000;

#[derive(Debug)]
enum Inst {
    Match,
    Char(char),
    Any(bool),
    Set(Class),
    Split(usize, usize),
    Jmp(usize),
    Save(usize),
    Start(bool),
    End(bool),
}

#[derive(Debug)]
pub(super) struct Program {
    insts: Vec<Inst>,
    pub groups: usize,
}

pub(super) struct Flags {
    pub dot_all: bool,
    pub multi_line: bool,
    pub case_insensitive: bool,
}

pub(super) fn compile(parsed: Parsed, flags: &Flags) -> Result<Program, RegexError> {
    let mut c = Compiler {
        insts: Vec::new(),
        flags,
    };
    c.push(Inst::Save(0))?;
    c.node(&parsed.node)?;
    c.push(Inst::Save(1))?;
    c.push(Inst::Match)?;
    Ok(Program {
        insts: c.insts,
        groups: parsed.groups,
    })
}

struct Compiler<'f> {
    insts: Vec<Inst>,
    flags: &'f Flags,
}

fn too_big() -> RegexError {
    RegexError::Unsupported(format!(
        "a regular expression whose program exceeds {MAX_PROGRAM} instructions (counted quantifiers are expanded)"
    ))
}

impl Compiler<'_> {
    fn push(&mut self, inst: Inst) -> Result<usize, RegexError> {
        if self.insts.len() >= MAX_PROGRAM {
            return Err(too_big());
        }
        self.insts.push(inst);
        Ok(self.insts.len() - 1)
    }

    fn matcher(&self, class: &Class) -> Result<Class, RegexError> {
        let mut class = class.clone();
        if self.flags.case_insensitive && !class.fold_case() {
            return Err(RegexError::Unsupported(
                "the `i` flag with a character range over 131072 characters".into(),
            ));
        }
        Ok(class)
    }

    fn node(&mut self, node: &Node) -> Result<(), RegexError> {
        match node {
            Node::Empty => {}
            Node::Char(c) => {
                if self.flags.case_insensitive && !(c.is_ascii() && !c.is_ascii_alphabetic()) {
                    let m = self.matcher(&Class::literal(*c))?;
                    self.push(Inst::Set(m))?;
                } else {
                    self.push(Inst::Char(*c))?;
                }
            }
            Node::Class(class) => {
                let m = self.matcher(class)?;
                self.push(Inst::Set(m))?;
            }
            Node::Any => {
                self.push(Inst::Any(self.flags.dot_all))?;
            }
            Node::Start => {
                self.push(Inst::Start(self.flags.multi_line))?;
            }
            Node::End => {
                self.push(Inst::End(self.flags.multi_line))?;
            }
            Node::Group(n, inner) => {
                self.push(Inst::Save(2 * n))?;
                self.node(inner)?;
                self.push(Inst::Save(2 * n + 1))?;
            }
            Node::Concat(parts) => {
                for p in parts {
                    self.node(p)?;
                }
            }
            Node::Alt(branches) => {
                let mut jumps = Vec::new();
                for (i, b) in branches.iter().enumerate() {
                    if i + 1 < branches.len() {
                        let split = self.push(Inst::Split(0, 0))?;
                        self.node(b)?;
                        jumps.push(self.push(Inst::Jmp(0))?);
                        let next = self.insts.len();
                        self.insts[split] = Inst::Split(split + 1, next);
                    } else {
                        self.node(b)?;
                    }
                }
                let end = self.insts.len();
                for j in jumps {
                    self.insts[j] = Inst::Jmp(end);
                }
            }
            Node::Repeat {
                node,
                min,
                max,
                greedy,
            } => {
                if *min as usize > MAX_PROGRAM || max.is_some_and(|m| m as usize > MAX_PROGRAM) {
                    return Err(too_big());
                }
                for _ in 0..*min {
                    self.node(node)?;
                }
                let split = |body: usize, out: usize| {
                    if *greedy {
                        Inst::Split(body, out)
                    } else {
                        Inst::Split(out, body)
                    }
                };
                match max {
                    None => {
                        let l = self.push(Inst::Split(0, 0))?;
                        self.node(node)?;
                        self.push(Inst::Jmp(l))?;
                        let out = self.insts.len();
                        self.insts[l] = split(l + 1, out);
                    }
                    Some(max) => {
                        let mut splits = Vec::new();
                        for _ in *min..*max {
                            splits.push(self.push(Inst::Split(0, 0))?);
                            self.node(node)?;
                        }
                        let out = self.insts.len();
                        for s in splits {
                            self.insts[s] = split(s + 1, out);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

const NONE: usize = usize::MAX;

struct Threads {
    pcs: Vec<usize>,
    /// `slots` entries per thread, parallel to `pcs`.
    caps: Vec<usize>,
    marks: Vec<usize>,
}

impl Threads {
    fn new(len: usize) -> Threads {
        Threads {
            pcs: Vec::new(),
            caps: Vec::new(),
            marks: vec![0; len],
        }
    }

    fn clear(&mut self) {
        self.pcs.clear();
        self.caps.clear();
    }
}

enum Task {
    Visit(usize),
    Restore(usize, usize),
}

impl Program {
    #[cfg(test)]
    pub fn insts_len(&self) -> usize {
        self.insts.len()
    }

    /// The step budget for one call over `len` characters of input.
    pub fn budget(&self, len: usize) -> u64 {
        (len as u64 + 1)
            .saturating_mul(self.insts.len() as u64)
            .saturating_mul(STEPS_PER_CELL)
            .saturating_add(100_000)
            .min(STEP_CEILING)
    }

    /// The leftmost match at or after `from`, as capture slots (`2 * n`,
    /// `2 * n + 1` are start and end of group `n`, `usize::MAX` = unset;
    /// only the first `slots` are tracked). `stop_at_first` ends the search at
    /// the first thread that reaches the end of the program (enough for a
    /// yes/no answer).
    pub fn search(
        &self,
        text: &[char],
        from: usize,
        slots: usize,
        stop_at_first: bool,
        budget: &mut u64,
    ) -> Result<Option<Vec<usize>>, RegexError> {
        let n = self.insts.len();
        let (mut clist, mut nlist) = (Threads::new(n), Threads::new(n));
        let mut scratch = vec![NONE; slots];
        let mut stack = Vec::new();
        let mut matched: Option<Vec<usize>> = None;
        for pos in from..=text.len() {
            if matched.is_none() {
                scratch.iter_mut().for_each(|s| *s = NONE);
                self.add(&mut clist, 0, pos, text, &mut scratch, &mut stack, budget)?;
            } else if clist.pcs.is_empty() {
                break;
            }
            let c = text.get(pos).copied();
            nlist.clear();
            for i in 0..clist.pcs.len() {
                if *budget == 0 {
                    return Err(budget_error());
                }
                *budget -= 1;
                let pc = clist.pcs[i];
                let accepts = match (&self.insts[pc], c) {
                    (Inst::Match, _) => {
                        matched = Some(clist.caps[i * slots..(i + 1) * slots].to_vec());
                        if stop_at_first {
                            return Ok(matched);
                        }
                        break;
                    }
                    (_, None) => false,
                    (Inst::Char(x), Some(c)) => *x == c,
                    (Inst::Any(dot_all), Some(c)) => *dot_all || (c != '\n' && c != '\r'),
                    (Inst::Set(m), Some(c)) => m.eval(c).ok_or_else(|| undecided(c))?,
                    _ => unreachable!("only consuming instructions and Match are threads"),
                };
                if accepts {
                    scratch.copy_from_slice(&clist.caps[i * slots..(i + 1) * slots]);
                    self.add(
                        &mut nlist,
                        pc + 1,
                        pos + 1,
                        text,
                        &mut scratch,
                        &mut stack,
                        budget,
                    )?;
                }
            }
            std::mem::swap(&mut clist, &mut nlist);
        }
        Ok(matched)
    }

    /// Follows the empty transitions from `pc` at `pos` and queues the threads
    /// that wait for a character or have matched, in priority order.
    #[allow(clippy::too_many_arguments)]
    fn add(
        &self,
        list: &mut Threads,
        pc: usize,
        pos: usize,
        text: &[char],
        caps: &mut [usize],
        stack: &mut Vec<Task>,
        budget: &mut u64,
    ) -> Result<(), RegexError> {
        let stamp = pos + 1;
        stack.push(Task::Visit(pc));
        while let Some(task) = stack.pop() {
            let pc = match task {
                Task::Restore(slot, old) => {
                    caps[slot] = old;
                    continue;
                }
                Task::Visit(pc) => pc,
            };
            if list.marks[pc] == stamp {
                continue;
            }
            list.marks[pc] = stamp;
            if *budget == 0 {
                return Err(budget_error());
            }
            *budget -= 1;
            match &self.insts[pc] {
                Inst::Jmp(t) => stack.push(Task::Visit(*t)),
                Inst::Split(a, b) => {
                    stack.push(Task::Visit(*b));
                    stack.push(Task::Visit(*a));
                }
                Inst::Save(slot) => {
                    if *slot < caps.len() {
                        stack.push(Task::Restore(*slot, caps[*slot]));
                        caps[*slot] = pos;
                    }
                    stack.push(Task::Visit(pc + 1));
                }
                Inst::Start(multi) => {
                    if pos == 0 || (*multi && text[pos - 1] == '\n') {
                        stack.push(Task::Visit(pc + 1));
                    }
                }
                Inst::End(multi) => {
                    if pos == text.len() || (*multi && text[pos] == '\n') {
                        stack.push(Task::Visit(pc + 1));
                    }
                }
                Inst::Char(_) | Inst::Any(_) | Inst::Set(_) | Inst::Match => {
                    list.pcs.push(pc);
                    list.caps.extend_from_slice(caps);
                }
            }
        }
        Ok(())
    }
}

fn budget_error() -> RegexError {
    RegexError::Unsupported(format!(
        "a regular-expression evaluation beyond its step budget ({STEPS_PER_CELL} per instruction and character, at most {STEP_CEILING}; stopped, not guessed)"
    ))
}

fn undecided(c: char) -> RegexError {
    RegexError::Unsupported(format!(
        "deciding whether U+{:04X} is in a character class: the general category, name-character or case-mapping data for it is not available (see docs/packages/xpath-eval.md)",
        c as u32
    ))
}
