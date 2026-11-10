//! Recursive-descent parser for the XPath 2.0 subset (XPath 2.0 §A.1). XPath 2.0
//! has no fixed token set (`and`, `div`, `if`, `for` are names or operators
//! depending on position), so this parser reads the source directly instead of
//! going through the 1.0 lexer.

use super::VersionedError;
use super::ast::{AxisStep, Expr, NodeComparison, SetOp};
use super::atomic::{Arithmetic, Atomic, AtomicType, Comparison};
use super::decimal::Decimal;
use crate::ast::{Axis, NodeTest, QName};
use crate::parser::ParseError;

/// Words that are operators when they follow an operand. A lone `/` followed
/// by one of them is the root, not the start of a path (XPath 2.0 §A.1.2).
const OPERATOR_WORDS: [&str; 22] = [
    "and",
    "or",
    "div",
    "idiv",
    "mod",
    "union",
    "intersect",
    "except",
    "eq",
    "ne",
    "lt",
    "le",
    "gt",
    "ge",
    "is",
    "to",
    "instance",
    "treat",
    "castable",
    "cast",
    "satisfies",
    "return",
];

/// Kind tests this crate does not implement (XPath 2.0 §2.5.3).
const UNSUPPORTED_KIND_TESTS: [&str; 7] = [
    "element",
    "attribute",
    "document-node",
    "schema-element",
    "schema-attribute",
    "item",
    "empty-sequence",
];

pub(super) fn parse(source: &str) -> Result<Expr, VersionedError> {
    let mut p = Parser {
        src: source,
        pos: 0,
        depth: 0,
        spine: 0,
    };
    p.skip_ws()?;
    if p.at_end() {
        return Err(p.error("empty expression"));
    }
    let expr = p.expr()?;
    p.skip_ws()?;
    if !p.at_end() {
        return Err(p.error("unexpected trailing input"));
    }
    Ok(expr)
}

/// What one level of nesting costs against [`MAX_DEPTH`]: the recursive-descent
/// parser needs about twenty stack frames per level, far more than the evaluator.
const NEST_WEIGHT: usize = 8;

/// Largest syntax-tree depth the parser accepts (nesting weighted by `NEST_WEIGHT` per parenthesis,
/// argument list, predicate or branch, plus the length of the
/// longest left-leaning operator chain), so the recursive evaluator and the
/// recursive drop of the tree stay inside a debug-build stack.
pub(super) const MAX_DEPTH: usize = 256;

/// Longest `or`/`and` chain; the evaluator walks these without recursion.
const MAX_LOGICAL_CHAIN: usize = 4096;

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    depth: usize,
    spine: usize,
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.')
}

fn step(axis: Axis, test: NodeTest) -> Expr {
    Expr::Step(AxisStep {
        axis,
        test,
        predicates: Vec::new(),
    })
}

fn boxed(e: Expr) -> Box<Expr> {
    Box::new(e)
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn error(&self, message: &str) -> VersionedError {
        VersionedError::Parse(ParseError::new(self.pos, message))
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Skips whitespace and (nested) comments `(: ... :)` (XPath 2.0 §A.2.3.1).
    fn skip_ws(&mut self) -> Result<(), VersionedError> {
        loop {
            let trimmed = self.rest().trim_start_matches([' ', '\t', '\n', '\r']);
            self.pos = self.src.len() - trimmed.len();
            if !self.rest().starts_with("(:") {
                return Ok(());
            }
            let start = self.pos;
            let mut depth = 0usize;
            loop {
                let rest = self.rest();
                if rest.starts_with("(:") {
                    depth += 1;
                    self.pos += 2;
                } else if rest.starts_with(":)") {
                    depth -= 1;
                    self.pos += 2;
                    if depth == 0 {
                        break;
                    }
                } else if let Some(c) = rest.chars().next() {
                    self.pos += c.len_utf8();
                } else {
                    self.pos = start;
                    return Err(self.error("unterminated comment"));
                }
            }
        }
    }

    /// Consumes `symbol` if it comes next.
    fn eat(&mut self, symbol: &str) -> Result<bool, VersionedError> {
        self.skip_ws()?;
        if self.rest().starts_with(symbol) {
            self.pos += symbol.len();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, symbol: &str) -> Result<(), VersionedError> {
        if self.eat(symbol)? {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{symbol}`")))
        }
    }

    /// Whether the keyword `word` comes next (not merely a name that starts
    /// with it).
    fn at_word(&mut self, word: &str) -> Result<bool, VersionedError> {
        self.skip_ws()?;
        let rest = self.rest();
        Ok(rest.starts_with(word) && !rest[word.len()..].chars().next().is_some_and(is_name_char))
    }

    fn eat_word(&mut self, word: &str) -> Result<bool, VersionedError> {
        if self.at_word(word)? {
            self.pos += word.len();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect_word(&mut self, word: &str) -> Result<(), VersionedError> {
        if self.eat_word(word)? {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{word}`")))
        }
    }

    /// The first non-whitespace character after the keyword at the current
    /// position (comments are not looked through).
    fn char_after_word(&self, word: &str) -> Option<char> {
        self.rest()[word.len()..]
            .trim_start_matches([' ', '\t', '\n', '\r'])
            .chars()
            .next()
    }

    /// Reads a QName at `from` without consuming it: prefix, local part and
    /// the offset after it. `prefix:*` is not a QName.
    fn scan_qname(&self, from: usize) -> Option<(Option<String>, String, usize)> {
        let scan_ncname = |at: usize| -> Option<usize> {
            let rest = &self.src[at..];
            let mut chars = rest.char_indices();
            let (_, first) = chars.next()?;
            if !is_name_start(first) {
                return None;
            }
            let end = chars
                .find(|&(_, c)| !is_name_char(c))
                .map_or(rest.len(), |(i, _)| i);
            Some(at + end)
        };
        let first_end = scan_ncname(from)?;
        let first = &self.src[from..first_end];
        let after = &self.src[first_end..];
        if after.starts_with(':') && !after.starts_with("::") {
            let second_start = first_end + 1;
            if let Some(second_end) = scan_ncname(second_start) {
                return Some((
                    Some(first.to_owned()),
                    self.src[second_start..second_end].to_owned(),
                    second_end,
                ));
            }
        }
        Some((None, first.to_owned(), first_end))
    }

    fn qname(&mut self) -> Result<QName, VersionedError> {
        self.skip_ws()?;
        match self.scan_qname(self.pos) {
            Some((prefix, local, end)) => {
                self.pos = end;
                Ok(QName { prefix, local })
            }
            None => Err(self.error("expected a name")),
        }
    }

    // ---- Expr, ExprSingle (§3.1, §3.7-§3.9) --------------------------------

    fn expr(&mut self) -> Result<Expr, VersionedError> {
        let first = self.expr_single()?;
        if !self.eat(",")? {
            return Ok(first);
        }
        let mut items = vec![first];
        loop {
            items.push(self.expr_single()?);
            if !self.eat(",")? {
                return Ok(Expr::Sequence(items));
            }
        }
    }

    fn too_deep(&self) -> VersionedError {
        self.error(&format!(
            "expression is nested or chained too deeply (limit {MAX_DEPTH})"
        ))
    }

    /// One more link in a left-leaning operator chain.
    fn extend_spine(&mut self) -> Result<(), VersionedError> {
        self.spine += 1;
        if self.depth + self.spine > MAX_DEPTH {
            return Err(self.too_deep());
        }
        Ok(())
    }

    fn logical_link(&self, chain: &mut usize) -> Result<(), VersionedError> {
        *chain += 1;
        if *chain > MAX_LOGICAL_CHAIN {
            return Err(self.error(&format!(
                "more than {MAX_LOGICAL_CHAIN} operands in one `or`/`and` chain"
            )));
        }
        Ok(())
    }

    fn expr_single(&mut self) -> Result<Expr, VersionedError> {
        self.depth += NEST_WEIGHT;
        if self.depth > MAX_DEPTH {
            return Err(self.too_deep());
        }
        let outer_spine = std::mem::replace(&mut self.spine, 0);
        let result = self.expr_single_inner();
        self.spine = outer_spine;
        self.depth -= NEST_WEIGHT;
        result
    }

    fn expr_single_inner(&mut self) -> Result<Expr, VersionedError> {
        self.skip_ws()?;
        if self.at_word("for")? && self.char_after_word("for") == Some('$') {
            self.pos += 3;
            let bindings = self.bindings()?;
            self.expect_word("return")?;
            let body = self.expr_single()?;
            return Ok(Expr::For(bindings, boxed(body)));
        }
        for (word, every) in [("some", false), ("every", true)] {
            if self.at_word(word)? && self.char_after_word(word) == Some('$') {
                self.pos += word.len();
                let bindings = self.bindings()?;
                self.expect_word("satisfies")?;
                let body = self.expr_single()?;
                return Ok(Expr::Quantified(every, bindings, boxed(body)));
            }
        }
        if self.at_word("if")? && self.char_after_word("if") == Some('(') {
            self.pos += 2;
            self.expect("(")?;
            let condition = self.expr()?;
            self.expect(")")?;
            self.expect_word("then")?;
            let then = self.expr_single()?;
            self.expect_word("else")?;
            let otherwise = self.expr_single()?;
            return Ok(Expr::If(boxed(condition), boxed(then), boxed(otherwise)));
        }
        self.or_expr()
    }

    /// `$v in ExprSingle (, $w in ExprSingle)*`
    fn bindings(&mut self) -> Result<Vec<(QName, Expr)>, VersionedError> {
        let mut bindings = Vec::new();
        loop {
            self.expect("$")?;
            let name = self.qname()?;
            self.expect_word("in")?;
            bindings.push((name, self.expr_single()?));
            self.extend_spine()?;
            if !self.eat(",")? {
                return Ok(bindings);
            }
        }
    }

    // ---- operators, loosest first (§3.6, §3.5, §3.3.1, §3.4, §3.3.3, §3.10) -

    fn or_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut chain = 0;
        let mut lhs = self.and_expr()?;
        while self.eat_word("or")? {
            self.logical_link(&mut chain)?;
            lhs = Expr::Or(boxed(lhs), boxed(self.and_expr()?));
        }
        Ok(lhs)
    }

    fn and_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut chain = 0;
        let mut lhs = self.comparison_expr()?;
        while self.eat_word("and")? {
            self.logical_link(&mut chain)?;
            lhs = Expr::And(boxed(lhs), boxed(self.comparison_expr()?));
        }
        Ok(lhs)
    }

    fn comparison_expr(&mut self) -> Result<Expr, VersionedError> {
        let lhs = self.range_expr()?;
        self.skip_ws()?;
        // Longest symbol first: `<<` and `<=` before `<`.
        for (symbol, op) in [
            ("<<", NodeComparison::Precedes),
            (">>", NodeComparison::Follows),
        ] {
            if self.eat(symbol)? {
                return Ok(Expr::Node(boxed(lhs), op, boxed(self.range_expr()?)));
            }
        }
        if self.eat_word("is")? {
            return Ok(Expr::Node(
                boxed(lhs),
                NodeComparison::Is,
                boxed(self.range_expr()?),
            ));
        }
        for (symbol, op) in [
            ("!=", Comparison::Ne),
            ("<=", Comparison::Le),
            (">=", Comparison::Ge),
            ("=", Comparison::Eq),
            ("<", Comparison::Lt),
            (">", Comparison::Gt),
        ] {
            if self.eat(symbol)? {
                return Ok(Expr::General(boxed(lhs), op, boxed(self.range_expr()?)));
            }
        }
        for (word, op) in [
            ("eq", Comparison::Eq),
            ("ne", Comparison::Ne),
            ("lt", Comparison::Lt),
            ("le", Comparison::Le),
            ("gt", Comparison::Gt),
            ("ge", Comparison::Ge),
        ] {
            if self.eat_word(word)? {
                return Ok(Expr::Value(boxed(lhs), op, boxed(self.range_expr()?)));
            }
        }
        Ok(lhs)
    }

    fn range_expr(&mut self) -> Result<Expr, VersionedError> {
        let lhs = self.additive_expr()?;
        if self.eat_word("to")? {
            return Ok(Expr::Range(boxed(lhs), boxed(self.additive_expr()?)));
        }
        Ok(lhs)
    }

    fn additive_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut lhs = self.multiplicative_expr()?;
        loop {
            let op = if self.eat("+")? {
                Arithmetic::Add
            } else if self.eat("-")? {
                Arithmetic::Sub
            } else {
                return Ok(lhs);
            };
            self.extend_spine()?;
            lhs = Expr::Arithmetic(boxed(lhs), op, boxed(self.multiplicative_expr()?));
        }
    }

    fn multiplicative_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut lhs = self.union_expr()?;
        loop {
            let op = if self.eat("*")? {
                Arithmetic::Mul
            } else if self.eat_word("div")? {
                Arithmetic::Div
            } else if self.eat_word("idiv")? {
                Arithmetic::IDiv
            } else if self.eat_word("mod")? {
                Arithmetic::Mod
            } else {
                return Ok(lhs);
            };
            self.extend_spine()?;
            lhs = Expr::Arithmetic(boxed(lhs), op, boxed(self.union_expr()?));
        }
    }

    fn union_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut lhs = self.intersect_expr()?;
        while self.eat("|")? || self.eat_word("union")? {
            self.extend_spine()?;
            lhs = Expr::SetOperation(boxed(lhs), SetOp::Union, boxed(self.intersect_expr()?));
        }
        Ok(lhs)
    }

    fn intersect_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut lhs = self.instance_of_expr()?;
        loop {
            let op = if self.eat_word("intersect")? {
                SetOp::Intersect
            } else if self.eat_word("except")? {
                SetOp::Except
            } else {
                return Ok(lhs);
            };
            self.extend_spine()?;
            lhs = Expr::SetOperation(boxed(lhs), op, boxed(self.instance_of_expr()?));
        }
    }

    fn instance_of_expr(&mut self) -> Result<Expr, VersionedError> {
        let operand = self.treat_expr()?;
        if self.at_word("instance")? {
            return Err(VersionedError::Unsupported("`instance of`".into()));
        }
        Ok(operand)
    }

    fn treat_expr(&mut self) -> Result<Expr, VersionedError> {
        let operand = self.castable_expr()?;
        if self.at_word("treat")? {
            return Err(VersionedError::Unsupported("`treat as`".into()));
        }
        Ok(operand)
    }

    fn castable_expr(&mut self) -> Result<Expr, VersionedError> {
        let operand = self.cast_expr()?;
        if self.eat_word("castable")? {
            self.expect_word("as")?;
            let (ty, optional) = self.single_type()?;
            return Ok(Expr::Cast(boxed(operand), ty, optional, true));
        }
        Ok(operand)
    }

    fn cast_expr(&mut self) -> Result<Expr, VersionedError> {
        let operand = self.unary_expr()?;
        if self.eat_word("cast")? {
            self.expect_word("as")?;
            let (ty, optional) = self.single_type()?;
            return Ok(Expr::Cast(boxed(operand), ty, optional, false));
        }
        Ok(operand)
    }

    /// `AtomicType '?'?` (XPath 2.0 §3.10.2).
    fn single_type(&mut self) -> Result<(AtomicType, bool), VersionedError> {
        let name = self.qname()?;
        let ty = match (
            name.prefix.as_deref(),
            AtomicType::from_local_name(&name.local),
        ) {
            (Some("xs"), Some(ty)) => ty,
            _ => {
                return Err(VersionedError::Unsupported(format!(
                    "atomic type `{}{}`",
                    name.prefix.map(|p| format!("{p}:")).unwrap_or_default(),
                    name.local
                )));
            }
        };
        Ok((ty, self.eat("?")?))
    }

    fn unary_expr(&mut self) -> Result<Expr, VersionedError> {
        let mut sign = None;
        loop {
            if self.eat("-")? {
                sign = Some(!sign.unwrap_or(false));
            } else if self.eat("+")? {
                sign = Some(sign.unwrap_or(false));
            } else {
                break;
            }
        }
        let operand = self.path_expr()?;
        Ok(match sign {
            Some(negative) => Expr::Unary(negative, boxed(operand)),
            None => operand,
        })
    }

    // ---- paths (§3.2) -----------------------------------------------------

    fn descendant_or_self() -> Expr {
        step(Axis::DescendantOrSelf, NodeTest::Node)
    }

    fn path_expr(&mut self) -> Result<Expr, VersionedError> {
        self.skip_ws()?;
        if self.rest().starts_with("//") {
            self.pos += 2;
            let left = Expr::Path(boxed(Expr::Root), boxed(Self::descendant_or_self()));
            return self.relative_path(Some(left));
        }
        if self.rest().starts_with('/') {
            self.pos += 1;
            return if self.starts_step()? {
                self.relative_path(Some(Expr::Root))
            } else {
                Ok(Expr::Root)
            };
        }
        self.relative_path(None)
    }

    /// Whether a step follows a lone `/` (XPath 2.0 §A.1.2: a lone `/` is the
    /// root when the next token is an operator word).
    fn starts_step(&mut self) -> Result<bool, VersionedError> {
        self.skip_ws()?;
        let Some(c) = self.peek() else {
            return Ok(false);
        };
        if is_name_start(c) {
            if let Some((None, local, end)) = self.scan_qname(self.pos)
                && OPERATOR_WORDS.contains(&local.as_str())
            {
                let next = self.src[end..]
                    .trim_start_matches([' ', '\t', '\n', '\r'])
                    .chars()
                    .next();
                return Ok(matches!(next, Some('(' | ':')));
            }
            return Ok(true);
        }
        Ok(matches!(c, '*' | '@' | '.' | '(' | '$' | '\'' | '"') || c.is_ascii_digit())
    }

    /// `StepExpr (('/' | '//') StepExpr)*`, continuing from `left` when the
    /// path began with `/` or `//`.
    fn relative_path(&mut self, left: Option<Expr>) -> Result<Expr, VersionedError> {
        let first = self.step_expr()?;
        let mut current = match left {
            Some(left) => Expr::Path(boxed(left), boxed(first)),
            None => first,
        };
        loop {
            self.skip_ws()?;
            if self.rest().starts_with("//") {
                self.pos += 2;
                current = Expr::Path(boxed(current), boxed(Self::descendant_or_self()));
            } else if self.rest().starts_with('/') {
                self.pos += 1;
            } else {
                return Ok(current);
            }
            self.extend_spine()?;
            current = Expr::Path(boxed(current), boxed(self.step_expr()?));
        }
    }

    fn step_expr(&mut self) -> Result<Expr, VersionedError> {
        self.skip_ws()?;
        let Some(c) = self.peek() else {
            return Err(self.error("unexpected end of expression, expected a step"));
        };
        if self.rest().starts_with("..") {
            self.pos += 2;
            return Ok(step(Axis::Parent, NodeTest::Node));
        }
        if c == '@' {
            self.pos += 1;
            let test = self.node_test()?;
            return self.axis_step_with_predicates(Axis::Attribute, test);
        }
        if c == '*' {
            let test = self.node_test()?;
            return self.axis_step_with_predicates(Axis::Child, test);
        }
        if is_name_start(c) {
            let Some((prefix, local, end)) = self.scan_qname(self.pos) else {
                return Err(self.error("expected a name"));
            };
            let after = self.src[end..].trim_start_matches([' ', '\t', '\n', '\r']);
            if prefix.is_none() && after.starts_with("::") {
                let Some(axis) = Axis::from_name(&local) else {
                    return Err(self.error(&format!("unknown axis `{local}`")));
                };
                if axis == Axis::Namespace {
                    return Err(VersionedError::Unsupported("the `namespace` axis".into()));
                }
                self.pos = self.src.len() - after.len() + 2;
                let test = self.node_test()?;
                return self.axis_step_with_predicates(axis, test);
            }
            if after.starts_with('(') {
                let kind_test = prefix.is_none()
                    && (matches!(
                        local.as_str(),
                        "node" | "text" | "comment" | "processing-instruction"
                    ) || UNSUPPORTED_KIND_TESTS.contains(&local.as_str()));
                if kind_test {
                    let test = self.node_test()?;
                    return self.axis_step_with_predicates(Axis::Child, test);
                }
                return self.postfix_expr();
            }
            let test = self.node_test()?;
            return self.axis_step_with_predicates(Axis::Child, test);
        }
        self.postfix_expr()
    }

    fn axis_step_with_predicates(
        &mut self,
        axis: Axis,
        test: NodeTest,
    ) -> Result<Expr, VersionedError> {
        let predicates = self.predicates()?;
        Ok(Expr::Step(AxisStep {
            axis,
            test,
            predicates,
        }))
    }

    fn predicates(&mut self) -> Result<Vec<Expr>, VersionedError> {
        let mut predicates = Vec::new();
        while self.eat("[")? {
            predicates.push(self.expr()?);
            self.expect("]")?;
        }
        Ok(predicates)
    }

    /// `NodeTest` (XPath 2.0 §3.2.1.1): name tests and the kind tests of 1.0.
    fn node_test(&mut self) -> Result<NodeTest, VersionedError> {
        self.skip_ws()?;
        if self.rest().starts_with('*') {
            self.pos += 1;
            if self.rest().starts_with(':') {
                return Err(VersionedError::Unsupported(
                    "the name test `*:local`".into(),
                ));
            }
            return Ok(NodeTest::AnyName);
        }
        let Some(c) = self.peek() else {
            return Err(self.error("unexpected end of expression, expected a node test"));
        };
        if !is_name_start(c) {
            return Err(self.error("expected a node test"));
        }
        // `prefix:*`
        let ncname_end = self.src[self.pos..]
            .char_indices()
            .find(|&(i, ch)| {
                !(if i == 0 {
                    is_name_start(ch)
                } else {
                    is_name_char(ch)
                })
            })
            .map_or(self.src.len(), |(i, _)| self.pos + i);
        if self.src[ncname_end..].starts_with(":*") {
            let prefix = self.src[self.pos..ncname_end].to_owned();
            self.pos = ncname_end + 2;
            return Ok(NodeTest::NamespaceWildcard(prefix));
        }
        let name = self.qname()?;
        if self.eat("(")? {
            if name.prefix.is_some() {
                return Err(self.error("a function call is not a node test"));
            }
            return match name.local.as_str() {
                "node" => self.close_kind_test(NodeTest::Node),
                "text" => self.close_kind_test(NodeTest::Text),
                "comment" => self.close_kind_test(NodeTest::Comment),
                "processing-instruction" => {
                    self.skip_ws()?;
                    let target = match self.peek() {
                        Some(q @ ('\'' | '"')) => Some(self.string_literal(q)?),
                        Some(c) if is_name_start(c) => Some(self.qname()?.local),
                        _ => None,
                    };
                    self.close_kind_test(NodeTest::ProcessingInstruction(target))
                }
                other => Err(VersionedError::Unsupported(format!(
                    "the kind test `{other}()`"
                ))),
            };
        }
        Ok(NodeTest::QName(name))
    }

    fn close_kind_test(&mut self, test: NodeTest) -> Result<NodeTest, VersionedError> {
        self.expect(")")?;
        Ok(test)
    }

    // ---- primary expressions (§3.1) ---------------------------------------

    fn postfix_expr(&mut self) -> Result<Expr, VersionedError> {
        let primary = self.primary_expr()?;
        let predicates = self.predicates()?;
        Ok(if predicates.is_empty() {
            primary
        } else {
            Expr::Filter(boxed(primary), predicates)
        })
    }

    fn primary_expr(&mut self) -> Result<Expr, VersionedError> {
        self.skip_ws()?;
        let Some(c) = self.peek() else {
            return Err(self.error("unexpected end of expression, expected an expression"));
        };
        match c {
            '\'' | '"' => Ok(Expr::Literal(Atomic::String(self.string_literal(c)?))),
            '$' => {
                self.pos += 1;
                Ok(Expr::Variable(self.qname()?))
            }
            '(' => {
                self.pos += 1;
                if self.eat(")")? {
                    return Ok(Expr::Sequence(Vec::new()));
                }
                let inner = self.expr()?;
                self.expect(")")?;
                Ok(inner)
            }
            '.' if !self.rest()[1..].starts_with(|d: char| d.is_ascii_digit()) => {
                self.pos += 1;
                Ok(Expr::ContextItem)
            }
            c if c.is_ascii_digit() || c == '.' => self.number(),
            c if is_name_start(c) => {
                let name = self.qname()?;
                self.expect("(")?;
                let mut args = Vec::new();
                if !self.eat(")")? {
                    loop {
                        args.push(self.expr_single()?);
                        if self.eat(")")? {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
                Ok(Expr::Call(name, args))
            }
            _ => Err(self.error("expected an expression")),
        }
    }

    /// A string literal; the quote character is escaped by doubling it
    /// (XPath 2.0 §A.2.1, `EscapeQuot`/`EscapeApos`).
    fn string_literal(&mut self, quote: char) -> Result<String, VersionedError> {
        let start = self.pos;
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                self.pos = start;
                return Err(self.error("unterminated string literal"));
            };
            self.pos += c.len_utf8();
            if c == quote {
                if self.peek() == Some(quote) {
                    self.pos += 1;
                    out.push(quote);
                } else {
                    return Ok(out);
                }
            } else {
                out.push(c);
            }
        }
    }

    /// `IntegerLiteral`, `DecimalLiteral` and `DoubleLiteral` (§3.1.1): the
    /// literal's form decides its type.
    fn number(&mut self) -> Result<Expr, VersionedError> {
        let start = self.pos;
        let bytes = self.src.as_bytes();
        let digits = |from: usize| {
            bytes[from..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        };
        let mut end = start + digits(start);
        let mut has_point = false;
        if bytes.get(end) == Some(&b'.') {
            has_point = true;
            end += 1;
            end += digits(end);
        }
        let mut is_double = false;
        if matches!(bytes.get(end), Some(b'e' | b'E')) {
            let mut e = end + 1;
            if matches!(bytes.get(e), Some(b'+' | b'-')) {
                e += 1;
            }
            let exponent_digits = digits(e);
            if exponent_digits > 0 {
                is_double = true;
                end = e + exponent_digits;
            }
        }
        let text = &self.src[start..end];
        self.pos = end;
        if is_double {
            let value: f64 = text
                .parse()
                .map_err(|_| self.error("invalid numeric literal"))?;
            return Ok(Expr::Literal(Atomic::Double(value)));
        }
        if has_point {
            let value = Decimal::parse(text).map_err(|_| {
                VersionedError::Unsupported("a decimal literal beyond 38 digits".into())
            })?;
            return Ok(Expr::Literal(Atomic::Decimal(value)));
        }
        let value: i64 = text
            .parse()
            .map_err(|_| VersionedError::Unsupported("an integer literal beyond 64 bits".into()))?;
        Ok(Expr::Literal(Atomic::Integer(value)))
    }
}
