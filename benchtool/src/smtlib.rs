use std::collections::{BTreeMap, BTreeSet};

use dashu::{float::DBig, integer::UBig};

use egglog::{
    EGraph,
    ast::{Action, Command, Expr, Fact, Literal, Rewrite},
};
use yaspar::{
    action::{
        ActionOnAttribute, ActionOnConstant, ActionOnIdentifier, ActionOnIndex, ActionOnSort,
        ActionOnString, ActionOnTerm, ParsingAction, ParsingResult, Pattern,
    },
    ast::{DatatypeDec, DatatypeDef, FunctionDef, Keyword},
    position::Range,
    smtlib2, tokenize_str,
};

#[derive(Clone)]
struct FunctionSignature {
    inputs: Vec<String>,
    output: String,
}

#[derive(Default)]
struct Signature {
    sorts: BTreeSet<String>,
    functions: BTreeMap<String, FunctionSignature>,
    predicates: BTreeMap<String, Vec<String>>,
}

/// Export egglog equations as quantified SMT-LIB assertions over
/// uninterpreted sorts and functions.
pub fn egglog_to_smtlib(source: &str, filename: Option<String>) -> Result<String, String> {
    let commands = EGraph::default()
        .parse_program(filename, source)
        .map_err(|error| error.to_string())?;
    let signature = Signature::from_commands(&commands)?;
    let mut output = Vec::new();

    for sort in &signature.sorts {
        output.push(format!("(declare-sort {} 0)", smt_symbol(sort)?));
    }
    for (name, function) in &signature.functions {
        output.push(format!(
            "(declare-fun {} ({}) {})",
            smt_symbol(name)?,
            function
                .inputs
                .iter()
                .map(|sort| smt_sort(sort))
                .collect::<Result<Vec<_>, _>>()?
                .join(" "),
            smt_sort(&function.output)?
        ));
    }
    for (name, inputs) in &signature.predicates {
        output.push(format!(
            "(declare-fun {} ({}) Bool)",
            smt_symbol(name)?,
            inputs
                .iter()
                .map(|sort| smt_sort(sort))
                .collect::<Result<Vec<_>, _>>()?
                .join(" ")
        ));
    }

    for command in &commands {
        match command {
            Command::Rewrite(_, rewrite, _) | Command::BiRewrite(_, rewrite) => {
                output.push(rewrite_to_assertion(rewrite, &signature)?);
            }
            Command::Rule { rule } => {
                if rule.head.0.len() != 1 {
                    return Err("SMT-LIB export requires rules to have exactly one action".into());
                }
                let mut renderer = Renderer::new(&signature);
                let head = match &rule.head.0[0] {
                    Action::Union(_, lhs, rhs) => renderer.equality(lhs, rhs)?,
                    Action::Expr(_, expression) => renderer.predicate(expression)?,
                    _ => return Err("unsupported egglog rule action for SMT-LIB export".into()),
                };
                let conditions = rule
                    .body
                    .iter()
                    .map(|fact| renderer.fact(fact))
                    .collect::<Result<Vec<_>, _>>()?;
                output.push(renderer.assertion(implies(&conditions, &head))?);
            }
            Command::Action(Action::Union(_, lhs, rhs)) => {
                let mut renderer = Renderer::new(&signature);
                let equality = renderer.equality(lhs, rhs)?;
                output.push(renderer.assertion(equality)?);
            }
            Command::Action(Action::Let(_, name, expression)) => {
                let function = signature
                    .functions
                    .get(name)
                    .expect("let bindings are collected in the signature");
                let mut renderer = Renderer::new(&signature);
                let term = renderer.term(expression, Some(&function.output))?.0;
                output.push(renderer.assertion(format!("(= {} {term})", smt_symbol(name)?))?);
            }
            Command::Action(Action::Expr(_, expression)) => {
                let mut renderer = Renderer::new(&signature);
                let predicate = renderer.predicate(expression)?;
                output.push(renderer.assertion(predicate)?);
            }
            _ => {}
        }
    }

    output.push("(check-sat)".into());
    Ok(format!("{}\n", output.join("\n")))
}

impl Signature {
    fn from_commands(commands: &[Command]) -> Result<Self, String> {
        let mut signature = Self::default();
        for command in commands {
            match command {
                Command::Sort {
                    name,
                    presort_and_args: None,
                    ..
                } => {
                    signature.sorts.insert(name.clone());
                }
                Command::Sort { name, .. } => {
                    return Err(format!(
                        "container sort {name} cannot be exported to SMT-LIB"
                    ));
                }
                Command::Datatype { name, variants, .. } => {
                    // Deliberately model egglog datatypes as uninterpreted sorts/functions.
                    signature.sorts.insert(name.clone());
                    for variant in variants {
                        signature.add_function(
                            &variant.name,
                            FunctionSignature {
                                inputs: variant.types.clone(),
                                output: name.clone(),
                            },
                        )?;
                    }
                }
                Command::Datatypes { .. } => {
                    return Err("mutually recursive datatypes are not supported yet".into());
                }
                Command::Constructor { name, schema, .. }
                | Command::Function { name, schema, .. } => {
                    signature.add_function(
                        name,
                        FunctionSignature {
                            inputs: schema.input.clone(),
                            output: schema.output.clone(),
                        },
                    )?;
                }
                Command::Relation { name, inputs, .. } => {
                    if signature
                        .predicates
                        .insert(name.clone(), inputs.clone())
                        .is_some()
                    {
                        return Err(format!("duplicate relation declaration: {name}"));
                    }
                }
                Command::Action(Action::Let(_, name, expression)) => {
                    let output = signature
                        .expression_sort(expression)
                        .ok_or_else(|| format!("cannot infer the sort of global binding {name}"))?;
                    signature.add_function(
                        name,
                        FunctionSignature {
                            inputs: Vec::new(),
                            output,
                        },
                    )?;
                }
                _ => {}
            }
        }
        Ok(signature)
    }

    fn add_function(&mut self, name: &str, function: FunctionSignature) -> Result<(), String> {
        if self.functions.insert(name.to_owned(), function).is_some() {
            return Err(format!("duplicate function declaration: {name}"));
        }
        Ok(())
    }

    fn expression_sort(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::Var(_, name) => self
                .functions
                .get(name)
                .filter(|function| function.inputs.is_empty())
                .map(|function| function.output.clone()),
            Expr::Call(_, name, _) => self
                .functions
                .get(name)
                .map(|function| function.output.clone()),
            Expr::Lit(_, Literal::Int(_)) => Some("i64".into()),
            Expr::Lit(_, Literal::Float(_)) => Some("f64".into()),
            Expr::Lit(_, Literal::String(_)) => Some("String".into()),
            Expr::Lit(_, Literal::Bool(_)) => Some("bool".into()),
            Expr::Lit(_, Literal::Unit) => None,
        }
    }
}

fn rewrite_to_assertion(rewrite: &Rewrite, signature: &Signature) -> Result<String, String> {
    let mut renderer = Renderer::new(signature);
    let equality = renderer.equality(&rewrite.lhs, &rewrite.rhs)?;
    let conditions = rewrite
        .conditions
        .iter()
        .map(|fact| renderer.fact(fact))
        .collect::<Result<Vec<_>, _>>()?;
    renderer.assertion(implies(&conditions, &equality))
}

fn implies(conditions: &[String], conclusion: &str) -> String {
    match conditions {
        [] => conclusion.to_owned(),
        [condition] => format!("(=> {condition} {conclusion})"),
        _ => format!("(=> (and {}) {conclusion})", conditions.join(" ")),
    }
}

struct Renderer<'a> {
    signature: &'a Signature,
    variables: BTreeMap<String, String>,
}

impl<'a> Renderer<'a> {
    fn new(signature: &'a Signature) -> Self {
        Self {
            signature,
            variables: BTreeMap::new(),
        }
    }

    fn assertion(&self, formula: String) -> Result<String, String> {
        if self.variables.is_empty() {
            Ok(format!("(assert {formula})"))
        } else {
            let bindings = self
                .variables
                .iter()
                .map(|(variable, sort)| {
                    Ok(format!("({} {})", smt_symbol(variable)?, smt_sort(sort)?))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(format!(
                "(assert (forall ({}) {formula}))",
                bindings.join(" ")
            ))
        }
    }

    fn equality(&mut self, lhs: &Expr, rhs: &Expr) -> Result<String, String> {
        let sort = self
            .known_sort(lhs)
            .or_else(|| self.known_sort(rhs))
            .ok_or_else(|| {
                "cannot infer the sort of an equality between two variables".to_owned()
            })?;
        let lhs = self.term(lhs, Some(&sort))?.0;
        let rhs = self.term(rhs, Some(&sort))?.0;
        Ok(format!("(= {lhs} {rhs})"))
    }

    fn fact(&mut self, fact: &Fact) -> Result<String, String> {
        match fact {
            Fact::Eq(_, lhs, rhs) => self.equality(lhs, rhs),
            Fact::Fact(expression) => self.predicate(expression),
        }
    }

    fn predicate(&mut self, expression: &Expr) -> Result<String, String> {
        let Expr::Call(_, name, arguments) = expression else {
            return Err("an SMT-LIB predicate condition must be a function call".into());
        };
        let inputs = self
            .signature
            .predicates
            .get(name)
            .ok_or_else(|| format!("undeclared predicate: {name}"))?;
        if inputs.len() != arguments.len() {
            return Err(format!("predicate {name} has the wrong arity"));
        }
        let arguments = arguments
            .iter()
            .zip(inputs)
            .map(|(argument, sort)| self.term(argument, Some(sort)).map(|term| term.0))
            .collect::<Result<Vec<_>, _>>()?;
        if arguments.is_empty() {
            smt_symbol(name)
        } else {
            Ok(format!("({} {})", smt_symbol(name)?, arguments.join(" ")))
        }
    }

    fn term(
        &mut self,
        expression: &Expr,
        expected: Option<&str>,
    ) -> Result<(String, String), String> {
        match expression {
            Expr::Var(_, variable) => {
                if let Some(function) = self
                    .signature
                    .functions
                    .get(variable)
                    .filter(|function| function.inputs.is_empty())
                {
                    check_expected(expected, &function.output)?;
                    return Ok((smt_symbol(variable)?, function.output.clone()));
                }
                let sort = expected
                    .map(str::to_owned)
                    .or_else(|| self.variables.get(variable).cloned())
                    .ok_or_else(|| format!("cannot infer the sort of variable {variable}"))?;
                if let Some(previous) = self.variables.insert(variable.clone(), sort.clone())
                    && previous != sort
                {
                    return Err(format!(
                        "variable {variable} has conflicting sorts {previous} and {sort}"
                    ));
                }
                Ok((smt_symbol(variable)?, sort))
            }
            Expr::Call(_, function, arguments) => {
                let signature = self
                    .signature
                    .functions
                    .get(function)
                    .ok_or_else(|| format!("undeclared function: {function}"))?;
                if signature.inputs.len() != arguments.len() {
                    return Err(format!("function {function} has the wrong arity"));
                }
                check_expected(expected, &signature.output)?;
                let arguments = arguments
                    .iter()
                    .zip(&signature.inputs)
                    .map(|(argument, sort)| self.term(argument, Some(sort)).map(|term| term.0))
                    .collect::<Result<Vec<_>, _>>()?;
                let term = if arguments.is_empty() {
                    smt_symbol(function)?
                } else {
                    format!("({} {})", smt_symbol(function)?, arguments.join(" "))
                };
                Ok((term, signature.output.clone()))
            }
            Expr::Lit(_, literal) => {
                let (term, sort) = literal_to_smt(literal)?;
                check_expected(expected, &sort)?;
                Ok((term, sort))
            }
        }
    }

    fn known_sort(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::Var(_, variable) => self.variables.get(variable).cloned(),
            Expr::Call(_, function, _) => self
                .signature
                .functions
                .get(function)
                .map(|signature| signature.output.clone()),
            Expr::Lit(_, Literal::Int(_)) => Some("i64".into()),
            Expr::Lit(_, Literal::Float(_)) => Some("f64".into()),
            Expr::Lit(_, Literal::String(_)) => Some("String".into()),
            Expr::Lit(_, Literal::Bool(_)) => Some("bool".into()),
            Expr::Lit(_, Literal::Unit) => None,
        }
    }
}

fn check_expected(expected: Option<&str>, actual: &str) -> Result<(), String> {
    if let Some(expected) = expected
        && expected != actual
    {
        return Err(format!(
            "sort mismatch: expected {expected}, found {actual}"
        ));
    }
    Ok(())
}

fn literal_to_smt(literal: &Literal) -> Result<(String, String), String> {
    match literal {
        Literal::Int(value) if *value < 0 => {
            Ok((format!("(- {})", value.unsigned_abs()), "i64".into()))
        }
        Literal::Int(value) => Ok((value.to_string(), "i64".into())),
        Literal::Float(value) if value.into_inner().is_finite() => {
            let value = value.into_inner();
            let negative = value.is_sign_negative();
            let mut rendered = value.abs().to_string();
            if !rendered.contains(['.', 'e', 'E']) {
                rendered.push_str(".0");
            }
            if negative {
                rendered = format!("(- {rendered})");
            }
            Ok((rendered, "f64".into()))
        }
        Literal::Float(value) => Err(format!("non-finite float is not supported: {value}")),
        Literal::String(value) => Ok((
            format!("\"{}\"", value.replace('"', "\"\"")),
            "String".into(),
        )),
        Literal::Bool(value) => Ok((value.to_string(), "bool".into())),
        Literal::Unit => Err("unit literals are not supported by SMT-LIB export yet".into()),
    }
}

fn smt_sort(sort: &str) -> Result<String, String> {
    match sort {
        "i64" | "u64" => Ok("Int".into()),
        "f64" => Ok("Real".into()),
        "String" => Ok("String".into()),
        "bool" => Ok("Bool".into()),
        sort => smt_symbol(sort),
    }
}

fn smt_symbol(symbol: &str) -> Result<String, String> {
    if symbol.contains(['|', '\\']) {
        Err(format!("symbol cannot be represented in SMT-LIB: {symbol}"))
    } else {
        Ok(format!("|{symbol}|"))
    }
}

/// Import the quantifier-free/ universally quantified Horn-equation fragment
/// of SMT-LIB into egglog. The input is parsed by Yaspar; no S-expression
/// parsing is done here.
pub fn smtlib_to_egglog(source: &str) -> Result<String, String> {
    smtlib_to_egglog_with_mode(source, ImportMode::Typed)
}

/// Import SMT-LIB as a single, untyped egglog term sort. This accepts
/// parameterized SMT sorts such as bit-vectors and treats SMT operators and
/// literals as uninterpreted constructors over `Term`.
pub fn smtlib_to_egglog_untyped(source: &str) -> Result<String, String> {
    smtlib_to_egglog_with_mode(source, ImportMode::Untyped)
}

fn smtlib_to_egglog_with_mode(source: &str, mode: ImportMode) -> Result<String, String> {
    let mut action = SmtlibAction;
    let commands = smtlib2::ScriptParser::new()
        .parse(&mut action, tokenize_str(source, true))
        .map_err(|error| format!("invalid SMT-LIB syntax: {error}"))?;
    SmtlibImporter::new(commands, mode).into_egglog()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ImportMode {
    Typed,
    Untyped,
}

#[derive(Clone, Debug)]
enum SmtConstant {
    Numeral(String),
    Decimal(String),
    String(String),
    Bool(bool),
    Binary(String),
    Hexadecimal(String),
}

#[derive(Clone, Debug)]
enum SmtTerm {
    Constant(SmtConstant),
    Identifier(String),
    Application {
        name: String,
        arguments: Vec<SmtTerm>,
    },
    Let(Vec<(String, SmtTerm)>, Box<SmtTerm>),
    Forall(Vec<(String, String)>, Box<SmtTerm>),
    Exists,
    Lambda,
    Match,
}

#[derive(Clone, Debug)]
enum SmtCommand {
    Assert(SmtTerm),
    DeclareConst {
        name: String,
        sort: String,
    },
    DeclareFun {
        name: String,
        inputs: Vec<String>,
        output: String,
    },
    DeclareSort {
        name: String,
        arity: String,
    },
    Unsupported(&'static str),
    Ignored,
}

struct SmtlibAction;

fn ignored() -> ParsingResult<SmtCommand> {
    Ok(SmtCommand::Ignored)
}

fn unsupported(command: &'static str) -> ParsingResult<SmtCommand> {
    Ok(SmtCommand::Unsupported(command))
}

impl ActionOnString for SmtlibAction {
    type Str = String;

    fn on_string(&mut self, _range: Range, value: String) -> ParsingResult<Self::Str> {
        Ok(value)
    }
}

impl ActionOnConstant for SmtlibAction {
    type Constant = SmtConstant;

    fn on_constant_binary(
        &mut self,
        _range: Range,
        bytes: Vec<u8>,
        len: usize,
    ) -> ParsingResult<Self::Constant> {
        Ok(SmtConstant::Binary(format!(
            "#b{}",
            yaspar::binary_to_string(&bytes, len)
        )))
    }

    fn on_constant_hexadecimal(
        &mut self,
        _range: Range,
        bytes: Vec<u8>,
        len: usize,
    ) -> ParsingResult<Self::Constant> {
        Ok(SmtConstant::Hexadecimal(format!(
            "#x{}",
            yaspar::hex_to_string(&bytes, len)
        )))
    }

    fn on_constant_decimal(&mut self, _range: Range, value: DBig) -> ParsingResult<Self::Constant> {
        Ok(SmtConstant::Decimal(value.to_string()))
    }

    fn on_constant_numeral(&mut self, _range: Range, value: UBig) -> ParsingResult<Self::Constant> {
        Ok(SmtConstant::Numeral(value.to_string()))
    }

    fn on_constant_string(
        &mut self,
        _range: Range,
        value: Self::Str,
    ) -> ParsingResult<Self::Constant> {
        Ok(SmtConstant::String(value))
    }

    fn on_constant_bool(&mut self, _range: Range, value: bool) -> ParsingResult<Self::Constant> {
        Ok(SmtConstant::Bool(value))
    }
}

impl ActionOnIndex for SmtlibAction {
    type Index = String;

    fn on_index_numeral(&mut self, _range: Range, value: UBig) -> ParsingResult<Self::Index> {
        Ok(value.to_string())
    }

    fn on_index_symbol(&mut self, _range: Range, value: Self::Str) -> ParsingResult<Self::Index> {
        Ok(value)
    }

    fn on_index_hexadecimal(
        &mut self,
        _range: Range,
        bytes: Vec<u8>,
        len: usize,
    ) -> ParsingResult<Self::Index> {
        Ok(format!("#x{}", yaspar::hex_to_string(&bytes, len)))
    }
}

impl ActionOnIdentifier for SmtlibAction {
    type Identifier = String;

    fn on_identifier(
        &mut self,
        _range: Range,
        symbol: Self::Str,
        indices: Vec<Self::Index>,
    ) -> ParsingResult<Self::Identifier> {
        if indices.is_empty() {
            Ok(symbol)
        } else {
            Ok(format!("(_ {symbol} {})", indices.join(" ")))
        }
    }
}

impl ActionOnAttribute for SmtlibAction {
    type Term = SmtTerm;
    type Attribute = ();

    fn on_attribute_keyword(
        &mut self,
        _range: Range,
        _keyword: Keyword,
    ) -> ParsingResult<Self::Attribute> {
        Ok(())
    }

    fn on_attribute_constant(
        &mut self,
        _range: Range,
        _keyword: Keyword,
        _constant: Self::Constant,
    ) -> ParsingResult<Self::Attribute> {
        Ok(())
    }

    fn on_attribute_symbol(
        &mut self,
        _range: Range,
        _keyword: Keyword,
        _symbol: Self::Str,
    ) -> ParsingResult<Self::Attribute> {
        Ok(())
    }

    fn on_attribute_named(
        &mut self,
        _range: Range,
        _name: Self::Str,
    ) -> ParsingResult<Self::Attribute> {
        Ok(())
    }

    fn on_attribute_pattern(
        &mut self,
        _range: Range,
        _patterns: Vec<Self::Term>,
    ) -> ParsingResult<Self::Attribute> {
        Ok(())
    }
}

impl ActionOnSort for SmtlibAction {
    type Sort = String;

    fn on_sort(
        &mut self,
        _range: Range,
        identifier: Self::Identifier,
        arguments: Vec<Self::Sort>,
    ) -> ParsingResult<Self::Sort> {
        if arguments.is_empty() {
            Ok(identifier)
        } else {
            Ok(format!("({identifier} {})", arguments.join(" ")))
        }
    }
}

impl ActionOnTerm for SmtlibAction {
    fn on_term_constant(
        &mut self,
        _range: Range,
        constant: Self::Constant,
    ) -> ParsingResult<Self::Term> {
        Ok(SmtTerm::Constant(constant))
    }

    fn on_term_identifier(
        &mut self,
        _range: Range,
        identifier: Self::Identifier,
        sort: Option<Self::Sort>,
    ) -> ParsingResult<Self::Term> {
        if let Some(sort) = sort {
            Ok(SmtTerm::Application {
                name: format!("(as {identifier} {sort})"),
                arguments: Vec::new(),
            })
        } else {
            Ok(SmtTerm::Identifier(identifier))
        }
    }

    fn on_term_app(
        &mut self,
        _range: Range,
        identifier: Self::Identifier,
        sort: Option<Self::Sort>,
        arguments: Vec<Self::Term>,
    ) -> ParsingResult<Self::Term> {
        let name = match sort {
            Some(sort) => format!("(as {identifier} {sort})"),
            None => identifier,
        };
        Ok(SmtTerm::Application { name, arguments })
    }

    fn on_term_let(
        &mut self,
        _range: Range,
        bindings: Vec<(Self::Str, Self::Term)>,
        body: Self::Term,
    ) -> ParsingResult<Self::Term> {
        Ok(SmtTerm::Let(bindings, Box::new(body)))
    }

    fn on_term_lambda(
        &mut self,
        _range: Range,
        _names: Vec<(Self::Str, Self::Sort)>,
        _body: Self::Term,
    ) -> ParsingResult<Self::Term> {
        Ok(SmtTerm::Lambda)
    }

    fn on_term_exists(
        &mut self,
        _range: Range,
        _names: Vec<(Self::Str, Self::Sort)>,
        _body: Self::Term,
    ) -> ParsingResult<Self::Term> {
        Ok(SmtTerm::Exists)
    }

    fn on_term_forall(
        &mut self,
        _range: Range,
        names: Vec<(Self::Str, Self::Sort)>,
        body: Self::Term,
    ) -> ParsingResult<Self::Term> {
        Ok(SmtTerm::Forall(names, Box::new(body)))
    }

    fn on_term_match(
        &mut self,
        _range: Range,
        _scrutinee: Self::Term,
        _cases: Vec<(Pattern<Self::Str>, Self::Term)>,
    ) -> ParsingResult<Self::Term> {
        Ok(SmtTerm::Match)
    }

    fn on_term_annotated(
        &mut self,
        _range: Range,
        term: Self::Term,
        _attributes: Vec<Self::Attribute>,
    ) -> ParsingResult<Self::Term> {
        Ok(term)
    }
}

impl ParsingAction for SmtlibAction {
    type Command = SmtCommand;

    fn on_command_assert(
        &mut self,
        _range: Range,
        term: Self::Term,
    ) -> ParsingResult<Self::Command> {
        Ok(SmtCommand::Assert(term))
    }

    fn on_command_check_sat(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_check_sat_assuming(
        &mut self,
        _range: Range,
        _terms: Vec<Self::Term>,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_declare_const(
        &mut self,
        _range: Range,
        name: Self::Str,
        sort: Self::Sort,
    ) -> ParsingResult<Self::Command> {
        Ok(SmtCommand::DeclareConst { name, sort })
    }

    fn on_command_declare_datatype(
        &mut self,
        _range: Range,
        _name: Self::Str,
        _datatype: DatatypeDec<Self::Str, Self::Sort>,
    ) -> ParsingResult<Self::Command> {
        unsupported("declare-datatype")
    }

    fn on_command_declare_datatypes(
        &mut self,
        _range: Range,
        _definitions: Vec<DatatypeDef<Self::Str, Self::Sort>>,
    ) -> ParsingResult<Self::Command> {
        unsupported("declare-datatypes")
    }

    fn on_command_declare_fun(
        &mut self,
        _range: Range,
        name: Self::Str,
        inputs: Vec<Self::Sort>,
        output: Self::Sort,
    ) -> ParsingResult<Self::Command> {
        Ok(SmtCommand::DeclareFun {
            name,
            inputs,
            output,
        })
    }

    fn on_command_declare_sort(
        &mut self,
        _range: Range,
        name: Self::Str,
        arity: UBig,
    ) -> ParsingResult<Self::Command> {
        Ok(SmtCommand::DeclareSort {
            name,
            arity: arity.to_string(),
        })
    }

    fn on_command_declare_sort_parameter(
        &mut self,
        _range: Range,
        _name: Self::Str,
    ) -> ParsingResult<Self::Command> {
        unsupported("declare-sort-parameter")
    }

    fn on_command_define_const(
        &mut self,
        _range: Range,
        _name: Self::Str,
        _sort: Self::Sort,
        _term: Self::Term,
    ) -> ParsingResult<Self::Command> {
        unsupported("define-const")
    }

    fn on_command_define_fun(
        &mut self,
        _range: Range,
        _definition: FunctionDef<Self::Str, Self::Sort, Self::Term>,
    ) -> ParsingResult<Self::Command> {
        unsupported("define-fun")
    }

    fn on_command_define_fun_rec(
        &mut self,
        _range: Range,
        _definition: FunctionDef<Self::Str, Self::Sort, Self::Term>,
    ) -> ParsingResult<Self::Command> {
        unsupported("define-fun-rec")
    }

    fn on_command_define_funs_rec(
        &mut self,
        _range: Range,
        _definitions: Vec<FunctionDef<Self::Str, Self::Sort, Self::Term>>,
    ) -> ParsingResult<Self::Command> {
        unsupported("define-funs-rec")
    }

    fn on_command_define_sort(
        &mut self,
        _range: Range,
        _name: Self::Str,
        _params: Vec<Self::Str>,
        _sort: Self::Sort,
    ) -> ParsingResult<Self::Command> {
        unsupported("define-sort")
    }

    fn on_command_echo(
        &mut self,
        _range: Range,
        _value: Self::Str,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_exit(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_assertions(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_assignment(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_info(
        &mut self,
        _range: Range,
        _keyword: Keyword,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_model(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_option(
        &mut self,
        _range: Range,
        _keyword: Keyword,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_proof(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_unsat_assumptions(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_unsat_core(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_get_value(
        &mut self,
        _range: Range,
        _terms: Vec<Self::Term>,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_pop(&mut self, _range: Range, _level: UBig) -> ParsingResult<Self::Command> {
        unsupported("pop")
    }

    fn on_command_push(&mut self, _range: Range, _level: UBig) -> ParsingResult<Self::Command> {
        unsupported("push")
    }

    fn on_command_reset(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        unsupported("reset")
    }

    fn on_command_reset_assertions(&mut self, _range: Range) -> ParsingResult<Self::Command> {
        unsupported("reset-assertions")
    }

    fn on_command_set_info(
        &mut self,
        _range: Range,
        _attribute: Self::Attribute,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_set_logic(
        &mut self,
        _range: Range,
        _logic: Self::Str,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }

    fn on_command_set_option(
        &mut self,
        _range: Range,
        _attribute: Self::Attribute,
    ) -> ParsingResult<Self::Command> {
        ignored()
    }
}

#[derive(Default)]
struct Symbols {
    sorts: BTreeMap<String, String>,
    functions: BTreeMap<String, String>,
    function_arities: BTreeMap<String, usize>,
    relations: BTreeMap<String, String>,
    relation_arities: BTreeMap<String, usize>,
    used: BTreeSet<String>,
}

impl Symbols {
    fn add_sort(&mut self, name: &str) -> String {
        add_symbol(&mut self.sorts, &mut self.used, name)
    }

    fn add_function(&mut self, name: &str, arity: usize) -> String {
        let result = add_symbol(&mut self.functions, &mut self.used, name);
        self.function_arities.insert(name.into(), arity);
        result
    }

    fn add_implicit_function(&mut self, name: &str, arity: usize) -> String {
        let key = implicit_symbol_key(name, arity);
        if let Some(existing) = self.functions.get(&key) {
            return existing.clone();
        }
        let raw_base = egglog_symbol(name);
        let base = if self.used.contains(&raw_base) {
            format!("{}_{}", egglog_symbol(name), arity)
        } else {
            raw_base
        };
        let result = add_symbol_with_base(&mut self.functions, &mut self.used, &key, &base);
        self.function_arities.insert(key, arity);
        result
    }

    fn add_relation(&mut self, name: &str, arity: usize) -> String {
        let result = add_symbol(&mut self.relations, &mut self.used, name);
        self.relation_arities.insert(name.into(), arity);
        result
    }

    fn sort(&self, sort: &str) -> Result<String, String> {
        match sort {
            "Int" => Ok("i64".into()),
            "Real" => Ok("f64".into()),
            "String" => Ok("String".into()),
            "Bool" => Ok("bool".into()),
            _ if sort.starts_with('(') => Err(format!(
                "parameterized SMT-LIB sort {sort} is not supported by egglog import"
            )),
            _ => self
                .sorts
                .get(sort)
                .cloned()
                .ok_or_else(|| format!("undeclared SMT-LIB sort: {sort}")),
        }
    }

    fn function_arity(&self, name: &str) -> Option<usize> {
        self.function_arities.get(name).copied()
    }

    fn relation_arity(&self, name: &str) -> Option<usize> {
        self.relation_arities.get(name).copied()
    }
}

fn implicit_symbol_key(name: &str, arity: usize) -> String {
    format!("{name}\u{1f}{arity}")
}

fn add_symbol(
    map: &mut BTreeMap<String, String>,
    used: &mut BTreeSet<String>,
    name: &str,
) -> String {
    add_symbol_with_base(map, used, name, &egglog_symbol(name))
}

fn add_symbol_with_base(
    map: &mut BTreeMap<String, String>,
    used: &mut BTreeSet<String>,
    key: &str,
    base: &str,
) -> String {
    if let Some(existing) = map.get(key) {
        return existing.clone();
    }
    let mut candidate = base.into();
    let mut suffix = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    used.insert(candidate.clone());
    map.insert(key.into(), candidate.clone());
    candidate
}

fn egglog_symbol(name: &str) -> String {
    const RESERVED: &[&str] = &[
        "birewrite",
        "constructor",
        "datatype",
        "extract",
        "let",
        "relation",
        "rewrite",
        "rule",
        "run",
        "sort",
        "union",
    ];
    if !name.is_empty()
        && !name.starts_with('$')
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        && name
            .chars()
            .next()
            .is_some_and(|character| !character.is_ascii_digit())
        && !RESERVED.contains(&name)
    {
        name.into()
    } else {
        let encoded = name
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("smt_{encoded}")
    }
}

struct SmtlibImporter {
    commands: Vec<SmtCommand>,
    mode: ImportMode,
    symbols: Symbols,
    declarations: Vec<String>,
    formulas: Vec<String>,
}

impl SmtlibImporter {
    fn new(commands: Vec<SmtCommand>, mode: ImportMode) -> Self {
        Self {
            commands,
            mode,
            symbols: Symbols::default(),
            declarations: Vec::new(),
            formulas: Vec::new(),
        }
    }

    fn into_egglog(mut self) -> Result<String, String> {
        if let Some(command) = self.commands.iter().find_map(|command| match command {
            SmtCommand::Unsupported(command) => Some(*command),
            _ => None,
        }) {
            return Err(format!(
                "SMT-LIB command {command} is not supported by egglog import"
            ));
        }
        self.collect_declarations()?;
        for command in self.commands.clone() {
            if let SmtCommand::Assert(term) = command {
                let formulas = self.assertion(&term, &FormulaScope::default())?;
                self.formulas.extend(formulas);
            }
        }
        if self.declarations.is_empty() && self.formulas.is_empty() {
            Ok(String::new())
        } else {
            let mut output = self.declarations;
            output.extend(self.formulas);
            Ok(format!("{}\n", output.join("\n")))
        }
    }

    fn collect_declarations(&mut self) -> Result<(), String> {
        if self.mode == ImportMode::Untyped {
            self.symbols.add_sort("Term");
            self.declarations.push("(sort Term)".into());
        }
        for command in self.commands.clone() {
            match command {
                SmtCommand::DeclareSort { name, arity } => {
                    if self.mode == ImportMode::Untyped {
                        continue;
                    }
                    if arity != "0" {
                        return Err(format!(
                            "parameterized SMT-LIB sort {name}/{arity} is not supported"
                        ));
                    }
                    let name = self.symbols.add_sort(&name);
                    self.declarations.push(format!("(sort {name})"));
                }
                SmtCommand::DeclareConst { name, sort } => {
                    self.declare_function(&name, &[], &sort)?;
                }
                SmtCommand::DeclareFun {
                    name,
                    inputs,
                    output,
                } => self.declare_function(&name, &inputs, &output)?,
                SmtCommand::Assert(_) | SmtCommand::Unsupported(_) | SmtCommand::Ignored => {}
            }
        }
        Ok(())
    }

    fn declare_function(
        &mut self,
        name: &str,
        inputs: &[String],
        output: &str,
    ) -> Result<(), String> {
        let inputs = inputs
            .iter()
            .map(|sort| self.sort(sort))
            .collect::<Result<Vec<_>, _>>()?;
        if output == "Bool" {
            let name = self.symbols.add_relation(name, inputs.len());
            self.declarations
                .push(format!("(relation {name} ({}))", inputs.join(" ")));
        } else {
            let output = self.sort(output)?;
            let name = self.symbols.add_function(name, inputs.len());
            self.declarations.push(format!(
                "(constructor {name} ({}) {output})",
                inputs.join(" ")
            ));
        }
        Ok(())
    }

    fn sort(&self, sort: &str) -> Result<String, String> {
        if self.mode == ImportMode::Untyped {
            Ok("Term".into())
        } else {
            self.symbols.sort(sort)
        }
    }

    fn function(&mut self, name: &str, arity: usize) -> Result<String, String> {
        if let Some(function) = self.symbols.functions.get(name).cloned() {
            let expected = self.symbols.function_arity(name).expect("function arity");
            if expected != arity {
                return Err(format!(
                    "SMT-LIB function {name} has arity {arity}, but was declared with arity {expected}"
                ));
            }
            return Ok(function);
        }
        let implicit = implicit_symbol_key(name, arity);
        if let Some(function) = self.symbols.functions.get(&implicit).cloned() {
            return Ok(function);
        }
        if self.mode != ImportMode::Untyped {
            return Err(format!("undeclared SMT-LIB function: {name}"));
        }
        let function = self.symbols.add_implicit_function(name, arity);
        self.declarations.push(format!(
            "(constructor {function} ({}) Term)",
            std::iter::repeat_n("Term", arity)
                .collect::<Vec<_>>()
                .join(" ")
        ));
        Ok(function)
    }

    fn relation(&mut self, name: &str, arity: usize) -> Result<String, String> {
        if let Some(relation) = self.symbols.relations.get(name).cloned() {
            let expected = self.symbols.relation_arity(name).expect("relation arity");
            if expected != arity {
                return Err(format!(
                    "SMT-LIB relation {name} has arity {arity}, but was declared with arity {expected}"
                ));
            }
            return Ok(relation);
        }
        if self.mode != ImportMode::Untyped {
            return Err(format!("undeclared SMT-LIB relation: {name}"));
        }
        let relation = self.symbols.add_relation(name, arity);
        self.declarations.push(format!(
            "(relation {relation} ({}))",
            std::iter::repeat_n("Term", arity)
                .collect::<Vec<_>>()
                .join(" ")
        ));
        Ok(relation)
    }

    fn untyped_literal(&mut self, literal: &str) -> Result<String, String> {
        let name = if let Some(hexadecimal) = literal.strip_prefix("#x") {
            format!("hex_{hexadecimal}")
        } else if let Some(binary) = literal.strip_prefix("#b") {
            format!("bin_{binary}")
        } else {
            format!("literal:{literal}")
        };
        let function = self.function(&name, 0)?;
        Ok(format!("({function})"))
    }

    fn assertion(&mut self, term: &SmtTerm, scope: &FormulaScope) -> Result<Vec<String>, String> {
        match term {
            SmtTerm::Forall(bindings, body) => {
                let mut scope = scope.clone();
                for (name, sort) in bindings {
                    // Check that the sort is one we can represent even though egglog
                    // infers variable sorts from their use.
                    self.sort(sort)?;
                    scope.variables.insert(name.clone(), egglog_symbol(name));
                }
                self.assertion(body, &scope)
            }
            SmtTerm::Let(bindings, body) => {
                let mut scope = scope.clone();
                for (name, value) in bindings {
                    scope
                        .bindings
                        .insert(name.clone(), self.term(value, &scope)?);
                }
                self.assertion(body, &scope)
            }
            SmtTerm::Application { name, arguments } if name == "and" => arguments
                .iter()
                .map(|argument| self.assertion(argument, scope))
                .collect::<Result<Vec<_>, _>>()
                .map(|groups| groups.into_iter().flatten().collect()),
            SmtTerm::Application { name, arguments } if name == "=>" => {
                let [antecedent, consequent] = arguments.as_slice() else {
                    return Err("SMT-LIB implication must have exactly two arguments".into());
                };
                let conditions = self.conditions(antecedent, scope)?;
                self.conclusion(consequent, &conditions, scope)
            }
            SmtTerm::Application { name, arguments } if name == "or" => {
                self.horn_clause(arguments, scope)
            }
            _ => self.conclusion(term, &[], scope),
        }
    }

    fn horn_clause(
        &mut self,
        literals: &[SmtTerm],
        scope: &FormulaScope,
    ) -> Result<Vec<String>, String> {
        let mut conditions = Vec::new();
        let mut conclusion = None;
        for literal in literals {
            if let SmtTerm::Application { name, arguments } = literal
                && name == "not"
            {
                let [atom] = arguments.as_slice() else {
                    return Err("SMT-LIB not must have exactly one argument".into());
                };
                conditions.push(self.fact(atom, scope)?);
            } else if conclusion.replace(literal).is_some() {
                return Err("SMT-LIB clause is not Horn: expected one positive literal".into());
            }
        }
        let conclusion =
            conclusion.ok_or_else(|| "SMT-LIB clause has no positive literal".to_owned())?;
        self.conclusion(conclusion, &conditions, scope)
    }

    fn conditions(&mut self, term: &SmtTerm, scope: &FormulaScope) -> Result<Vec<String>, String> {
        match term {
            SmtTerm::Application { name, arguments } if name == "and" => arguments
                .iter()
                .map(|argument| self.fact(argument, scope))
                .collect(),
            _ => Ok(vec![self.fact(term, scope)?]),
        }
    }

    fn conclusion(
        &mut self,
        term: &SmtTerm,
        conditions: &[String],
        scope: &FormulaScope,
    ) -> Result<Vec<String>, String> {
        let suffix = if conditions.is_empty() {
            String::new()
        } else {
            format!(" :when ({})", conditions.join(" "))
        };
        match equality_arguments(term) {
            Some((lhs, rhs)) => Ok(vec![format!(
                "(birewrite {} {}{suffix})",
                self.term(lhs, scope)?,
                self.term(rhs, scope)?,
            )]),
            None => Ok(vec![format!(
                "(rule ({}) ({}))",
                conditions.join(" "),
                self.predicate(term, scope)?,
            )]),
        }
    }

    fn fact(&mut self, term: &SmtTerm, scope: &FormulaScope) -> Result<String, String> {
        match equality_arguments(term) {
            Some((lhs, rhs)) => Ok(format!(
                "(= {} {})",
                self.term(lhs, scope)?,
                self.term(rhs, scope)?,
            )),
            None => self.predicate(term, scope),
        }
    }

    fn predicate(&mut self, term: &SmtTerm, scope: &FormulaScope) -> Result<String, String> {
        let (name, arguments) = match term {
            SmtTerm::Identifier(name) => (name.as_str(), &[][..]),
            SmtTerm::Application { name, arguments } => (name.as_str(), arguments.as_slice()),
            _ => return Err("SMT-LIB predicate must be an application".into()),
        };
        let name = self.relation(name, arguments.len())?;
        let arguments = arguments
            .iter()
            .map(|argument| self.term(argument, scope))
            .collect::<Result<Vec<_>, _>>()?;
        if arguments.is_empty() {
            Ok(format!("({name})"))
        } else {
            Ok(format!("({name} {})", arguments.join(" ")))
        }
    }

    fn term(&mut self, term: &SmtTerm, scope: &FormulaScope) -> Result<String, String> {
        match term {
            SmtTerm::Constant(SmtConstant::Numeral(value) | SmtConstant::Decimal(value)) => {
                if self.mode == ImportMode::Untyped {
                    self.untyped_literal(value)
                } else {
                    Ok(value.clone())
                }
            }
            SmtTerm::Constant(SmtConstant::String(value)) => {
                if self.mode == ImportMode::Untyped {
                    self.untyped_literal(&format!("{value:?}"))
                } else {
                    Ok(format!("{value:?}"))
                }
            }
            SmtTerm::Constant(SmtConstant::Bool(value)) => {
                if self.mode == ImportMode::Untyped {
                    self.untyped_literal(&value.to_string())
                } else {
                    Ok(value.to_string())
                }
            }
            SmtTerm::Constant(SmtConstant::Binary(value) | SmtConstant::Hexadecimal(value)) => {
                if self.mode == ImportMode::Untyped {
                    self.untyped_literal(value)
                } else {
                    Err("SMT-LIB bit-vector literals are not supported by egglog import; use --untyped".into())
                }
            }
            SmtTerm::Identifier(name) => {
                if let Some(value) = scope.bindings.get(name) {
                    return Ok(value.clone());
                }
                if let Some(variable) = scope.variables.get(name) {
                    return Ok(variable.clone());
                }
                Ok(format!("({})", self.function(name, 0)?))
            }
            SmtTerm::Application { name, arguments }
                if self.mode == ImportMode::Typed && name == "-" && arguments.len() == 1 =>
            {
                let value = self.term(&arguments[0], scope)?;
                if value
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '.')
                {
                    Ok(format!("-{value}"))
                } else {
                    Err("SMT-LIB unary minus is only supported for numeric literals".into())
                }
            }
            SmtTerm::Application { name, arguments } => {
                let name = self.function(name, arguments.len())?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.term(argument, scope))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("({name} {})", arguments.join(" ")))
            }
            SmtTerm::Let(bindings, body) => {
                let mut scope = scope.clone();
                for (name, value) in bindings {
                    scope
                        .bindings
                        .insert(name.clone(), self.term(value, &scope)?);
                }
                self.term(body, &scope)
            }
            SmtTerm::Forall(..) => {
                Err("SMT-LIB quantifier cannot be used as an egglog term".into())
            }
            SmtTerm::Exists => Err("existential SMT-LIB formulas are not supported".into()),
            SmtTerm::Lambda => Err("SMT-LIB lambdas are not supported".into()),
            SmtTerm::Match => Err("SMT-LIB match expressions are not supported".into()),
        }
    }
}

#[derive(Clone, Default)]
struct FormulaScope {
    variables: BTreeMap<String, String>,
    bindings: BTreeMap<String, String>,
}

fn equality_arguments(term: &SmtTerm) -> Option<(&SmtTerm, &SmtTerm)> {
    let SmtTerm::Application { name, arguments } = term else {
        return None;
    };
    if name != "=" {
        return None;
    }
    match arguments.as_slice() {
        [lhs, rhs] => Some((lhs, rhs)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    use egglog::EGraph;
    use tptp::TPTPIterator;

    use super::{egglog_to_smtlib, smtlib_to_egglog, smtlib_to_egglog_untyped};
    use crate::{Format, convert};

    const SMTLIB_EQUATIONS: &str = r#"
        (declare-sort |Expr| 0)
        (declare-fun |Zero| () |Expr|)
        (declare-fun |Add| (|Expr| |Expr|) |Expr|)
        (declare-fun |Ready| (|Expr|) Bool)
        (assert (forall ((|x| |Expr|)) (= (|Add| |Zero| |x|) |x|)))
        (assert (forall ((|x| |Expr|))
            (=> (and (|Ready| |x|) (= |x| |Zero|))
                (= (|Add| |x| |Zero|) |x|))))
        (check-sat)
    "#;

    #[test]
    fn imports_smtlib_equations_with_yaspar() {
        let egglog = smtlib_to_egglog(SMTLIB_EQUATIONS).unwrap();
        assert_eq!(
            egglog,
            "(sort Expr)\n\
             (constructor Zero () Expr)\n\
             (constructor Add (Expr Expr) Expr)\n\
             (relation Ready (Expr))\n\
             (birewrite (Add (Zero) x) x)\n\
             (birewrite (Add x (Zero)) x :when ((Ready x) (= x (Zero))))\n"
        );
        EGraph::default()
            .parse_program(None, &egglog)
            .expect("SMT-LIB import should produce valid egglog");
    }

    #[test]
    fn imports_bitvectors_into_the_untyped_term_sort() {
        let egglog =
            smtlib_to_egglog_untyped(include_str!("../../benchmarks/bitvec/bv8.smt2")).unwrap();
        assert!(egglog.starts_with("(sort Term)\n"), "{egglog}");
        assert!(
            egglog.contains("(constructor bvadd (Term Term) Term)"),
            "{egglog}"
        );
        assert!(egglog.contains("(constructor hex_00 () Term)"), "{egglog}");
        assert!(
            egglog.contains("(birewrite (bvadd x y) (bvadd y x))"),
            "{egglog}"
        );
        assert!(
            egglog.contains("(birewrite (bvadd (bvadd x y) z) (bvadd x (bvadd y z)))"),
            "{egglog}"
        );
        EGraph::default()
            .parse_program(None, &egglog)
            .expect("untyped bit-vector import should produce valid egglog");
    }

    #[test]
    fn round_trips_exported_smtlib_and_reports_unsupported_commands() {
        let source = r#"
            (datatype Expr (Zero) (Add Expr Expr))
            (let $zero (Zero))
            (union $zero (Add (Zero) (Zero)))
        "#;
        let smtlib = egglog_to_smtlib(source, None).unwrap();
        let egglog = smtlib_to_egglog(&smtlib).unwrap();
        assert!(egglog.contains("smt_247a65726f"), "{egglog}");
        EGraph::default()
            .parse_program(None, &egglog)
            .expect("round-tripped SMT-LIB should produce valid egglog");

        let error = smtlib_to_egglog("(define-fun f ((x Int)) Int x)").unwrap_err();
        assert!(error.contains("define-fun"), "{error}");
    }

    #[test]
    fn routes_smtlib_through_egglog_to_tptp_and_eq_prog_opt() {
        for format in [Format::Cnf, Format::Fof] {
            let tptp = convert(SMTLIB_EQUATIONS, None, Format::Smtlib, format).unwrap();
            assert!(
                TPTPIterator::<()>::new(tptp.as_bytes()).all(|formula| formula.is_ok()),
                "{tptp}"
            );
            assert_accepts("eprover-ho", &["--syntax-only", "-"], &tptp);
            assert_accepts(
                "vampire",
                &["--mode", "clausify", "--input_syntax", "tptp"],
                &tptp,
            );
        }

        let simple = r#"
            (declare-sort S 0)
            (declare-fun a () S)
            (declare-fun f (S) S)
            (assert (forall ((x S)) (= (f x) x)))
        "#;
        let eq_prog_opt = convert(simple, None, Format::Smtlib, Format::EqProgOpt).unwrap();
        eq_prog_opt::parse::parse_decls(&eq_prog_opt)
            .expect("SMT-LIB import should route to eq-prog-opt");
    }

    #[test]
    fn exports_datatypes_as_uninterpreted_sorts_and_z3_accepts_them() {
        let egglog = r#"
            (datatype Expr (Zero) (Add Expr Expr))
            (relation Ready (Expr))
            (rewrite (Add (Zero) x) x)
            (rewrite (Add x (Zero)) x :when ((Ready x) (= x (Zero))))
            (let $zero (Zero))
            (union $zero (Add (Zero) (Zero)))
        "#;
        let smtlib = egglog_to_smtlib(egglog, None).unwrap();
        assert!(smtlib.contains("(declare-sort |Expr| 0)"), "{smtlib}");
        assert!(!smtlib.contains("declare-datatype"), "{smtlib}");
        assert!(smtlib.contains("(declare-fun |Add| (|Expr| |Expr|) |Expr|)"));
        assert!(smtlib.contains("(forall ((|x| |Expr|))"));
        assert!(smtlib.contains("(declare-fun |$zero| () |Expr|)"));
        assert!(!smtlib.contains("forall ((|$zero|"));
        assert_z3_accepts(&smtlib);
    }

    fn assert_z3_accepts(input: &str) {
        let mut child = Command::new("z3")
            .args(["-in", "-smt2"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not run z3");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "Z3 rejected generated SMT-LIB\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(!stdout.contains("(error"), "Z3 reported an error: {stdout}");
        assert!(
            stdout.contains("sat") || stdout.contains("unknown"),
            "unexpected Z3 output: {stdout}"
        );
    }

    fn assert_accepts(prover: &str, arguments: &[&str], input: &str) {
        let mut child = Command::new(prover)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("could not run {prover}: {error}"));
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{prover} rejected generated TPTP\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr),
        );
    }
}
