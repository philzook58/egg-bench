use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    process::{Command, Stdio},
};

use tptp::{
    TPTPIterator, cnf,
    common::{AtomicWord, DefinedTerm as CommonDefinedTerm},
    fof,
    top::{AnnotatedFormula, TPTPInput},
};

/// Import the Horn fragment of TPTP CNF into an untyped egglog term sort.
pub fn cnf_to_egglog(source: &str) -> Result<String, String> {
    let mut parser = TPTPIterator::<()>::new(source.as_bytes());
    let mut symbols = Symbols::default();
    let mut rules = Vec::new();

    for input in parser.by_ref() {
        let input = input.map_err(|_| "invalid TPTP syntax".to_owned())?;
        match input {
            TPTPInput::Annotated(annotated) => match *annotated {
                AnnotatedFormula::Cnf(cnf) => {
                    let name = cnf.0.name.to_string();
                    rules.push(import_clause(&name, &cnf.0.formula, &mut symbols)?);
                }
                _ => return Err("expected CNF input; use FOF import to clausify first".into()),
            },
            TPTPInput::Include(_) => {
                return Err("TPTP include directives are not supported yet".into());
            }
        }
    }

    if !parser.remaining.is_empty() {
        return Err("invalid TPTP syntax near the end of the input".into());
    }
    if rules.is_empty() {
        return Ok(String::new());
    }

    let mut output = vec!["(sort TptpTerm)".to_owned()];
    for (name, arity) in &symbols.functions {
        let inputs = std::iter::repeat_n("TptpTerm", *arity)
            .collect::<Vec<_>>()
            .join(" ");
        output.push(format!("(constructor {name} ({inputs}) TptpTerm)"));
    }
    for (name, arity) in &symbols.predicates {
        let inputs = std::iter::repeat_n("TptpTerm", *arity)
            .collect::<Vec<_>>()
            .join(" ");
        output.push(format!("(relation {name} ({inputs}))"));
    }
    output.push(String::new());
    output.extend(rules);
    Ok(format!("{}\n", output.join("\n")))
}

/// Clausify and Skolemize FOF with Vampire, then import the resulting CNF.
pub fn fof_to_egglog(source: &str) -> Result<String, String> {
    let mut child = Command::new("vampire")
        .args([
            "--mode",
            "clausify",
            "--input_syntax",
            "tptp",
            "--function_definition_elimination",
            "none",
            "--unused_predicate_definition_removal",
            "off",
            "--statistics",
            "none",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not run Vampire: {error}"))?;

    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(source.as_bytes())
        .map_err(|error| format!("could not send FOF input to Vampire: {error}"))?;
    let result = child
        .wait_with_output()
        .map_err(|error| format!("could not wait for Vampire: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "Vampire could not clausify the FOF input: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }

    let cnf = String::from_utf8(result.stdout)
        .map_err(|error| format!("Vampire returned non-UTF-8 output: {error}"))?;
    cnf_to_egglog(&cnf)
}

fn import_clause(
    name: &str,
    formula: &cnf::Formula<'_>,
    symbols: &mut Symbols,
) -> Result<String, String> {
    let disjunction = match formula {
        cnf::Formula::Disjunction(disjunction) | cnf::Formula::Parenthesised(disjunction) => {
            disjunction
        }
    };
    let mut variables = ImportVariables::default();
    let literals = disjunction
        .0
        .iter()
        .map(|literal| import_literal(literal, symbols, &mut variables))
        .collect::<Result<Vec<_>, _>>()?;
    let positive = literals
        .iter()
        .enumerate()
        .filter(|(_, literal)| literal.positive)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    if positive.len() != 1 {
        return Err(format!(
            "CNF clause {name} is not Horn: expected exactly one positive literal, found {}",
            positive.len()
        ));
    }

    let head = &literals[positive[0]].atom;
    let conditions = literals
        .iter()
        .filter(|literal| !literal.positive)
        .map(|literal| literal.atom.as_condition())
        .collect::<Vec<_>>();
    let conditions = match head {
        Atom::Predicate(_) => add_grounding_terms(head, conditions, symbols)?,
        Atom::Equality(..) => conditions,
    };
    let when = if conditions.is_empty() {
        String::new()
    } else {
        format!(" :when ({})", conditions.join(" "))
    };

    match head {
        Atom::Equality(lhs, rhs) => equality_to_egglog(lhs, rhs, &conditions, &when),
        Atom::Predicate(atom) => Ok(format!("(rule ({}) ({atom}))", conditions.join(" "))),
    }
}

fn add_grounding_terms(
    head: &Atom,
    mut conditions: Vec<String>,
    symbols: &Symbols,
) -> Result<Vec<String>, String> {
    let grounded = conditions
        .iter()
        .flat_map(|condition| egglog_variables(condition))
        .collect::<HashSet<_>>();
    let head_terms: Vec<&str> = match head {
        Atom::Equality(lhs, rhs) => vec![lhs, rhs],
        Atom::Predicate(atom) => vec![atom],
    };
    let mut missing = head_terms
        .iter()
        .flat_map(|term| egglog_variables(term))
        .filter(|variable| !grounded.contains(variable))
        .collect::<HashSet<_>>();

    let mut candidates = head_terms
        .iter()
        .flat_map(|term| function_subterms(term, symbols))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(egglog_variables(candidate).len()));

    for candidate in candidates {
        let variables = egglog_variables(&candidate);
        if variables.iter().any(|variable| missing.contains(variable)) {
            missing.retain(|variable| !variables.contains(variable));
            conditions.push(candidate);
        }
    }

    if missing.is_empty() {
        Ok(conditions)
    } else {
        let missing = missing.into_iter().collect::<Vec<_>>().join(", ");
        Err(format!(
            "CNF clause {head:?} has variables that cannot be grounded in egglog: {missing}"
        ))
    }
}

fn function_subterms(term: &str, symbols: &Symbols) -> Vec<String> {
    let mut starts = Vec::new();
    let mut subterms = Vec::new();
    for (index, character) in term.char_indices() {
        match character {
            '(' => starts.push(index),
            ')' => {
                let start = starts.pop().expect("generated egglog term is balanced");
                let subterm = &term[start..=index];
                if let Some(name) = subterm
                    .strip_prefix('(')
                    .and_then(|rest| rest.split_whitespace().next())
                    && symbols.functions.contains_key(name)
                {
                    subterms.push(subterm.to_owned());
                }
            }
            _ => {}
        }
    }
    subterms
}

fn equality_to_egglog(
    lhs: &str,
    rhs: &str,
    conditions: &[String],
    when: &str,
) -> Result<String, String> {
    let condition_variables = conditions
        .iter()
        .flat_map(|condition| egglog_variables(condition))
        .collect::<HashSet<_>>();
    let lhs_variables = egglog_variables(lhs);
    let rhs_variables = egglog_variables(rhs);

    let lhs_to_rhs = is_grounded_pattern(lhs)
        && rhs_variables.iter().all(|variable| {
            lhs_variables.contains(variable) || condition_variables.contains(variable)
        });
    let rhs_to_lhs = is_grounded_pattern(rhs)
        && lhs_variables.iter().all(|variable| {
            rhs_variables.contains(variable) || condition_variables.contains(variable)
        });

    match (lhs_to_rhs, rhs_to_lhs) {
        (true, true) => Ok(format!("(birewrite {lhs} {rhs}{when})")),
        (true, false) => Ok(format!("(rewrite {lhs} {rhs}{when})")),
        (false, true) => Ok(format!("(rewrite {rhs} {lhs}{when})")),
        (false, false) => Err(format!(
            "CNF equality {lhs} = {rhs} cannot be expressed as a grounded egglog rewrite"
        )),
    }
}

fn is_grounded_pattern(term: &str) -> bool {
    term.starts_with('(')
}

fn egglog_variables(term: &str) -> HashSet<String> {
    let mut variables = HashSet::new();
    let mut token_start = None;
    let mut previous_non_whitespace = None;

    for (index, character) in term.char_indices() {
        if character.is_ascii_alphanumeric() || character == '_' {
            token_start.get_or_insert(index);
            continue;
        }
        if let Some(start) = token_start.take()
            && previous_non_whitespace != Some('(')
        {
            variables.insert(term[start..index].to_owned());
        }
        if token_start.is_none() {
            previous_non_whitespace = Some('x');
        }
        if !character.is_whitespace() {
            previous_non_whitespace = Some(character);
        }
    }
    if let Some(start) = token_start
        && previous_non_whitespace != Some('(')
    {
        variables.insert(term[start..].to_owned());
    }
    variables
}

fn import_literal(
    literal: &cnf::Literal<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<SignedAtom, String> {
    match literal {
        cnf::Literal::Atomic(atom) => Ok(SignedAtom {
            positive: true,
            atom: import_atom(atom, symbols, variables)?,
        }),
        cnf::Literal::NegatedAtomic(atom) => Ok(SignedAtom {
            positive: false,
            atom: import_atom(atom, symbols, variables)?,
        }),
        cnf::Literal::Infix(infix) => Ok(SignedAtom {
            positive: false,
            atom: Atom::Equality(
                import_term(&infix.left, symbols, variables)?,
                import_term(&infix.right, symbols, variables)?,
            ),
        }),
    }
}

fn import_atom(
    atom: &fof::AtomicFormula<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<Atom, String> {
    match atom {
        fof::AtomicFormula::Plain(fof::PlainAtomicFormula(term)) => Ok(Atom::Predicate(
            import_plain_predicate(term, symbols, variables)?,
        )),
        fof::AtomicFormula::Defined(fof::DefinedAtomicFormula::Infix(infix)) => Ok(Atom::Equality(
            import_term(&infix.left, symbols, variables)?,
            import_term(&infix.right, symbols, variables)?,
        )),
        fof::AtomicFormula::Defined(fof::DefinedAtomicFormula::Plain(
            fof::DefinedPlainFormula(term),
        )) => Ok(Atom::Predicate(import_defined_predicate(
            term, symbols, variables,
        )?)),
        fof::AtomicFormula::System(fof::SystemAtomicFormula(term)) => Ok(Atom::Predicate(
            import_system_predicate(term, symbols, variables)?,
        )),
    }
}

fn import_term(
    term: &fof::Term<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::Term::Variable(variable) => Ok(variables.get(variable.0.0)),
        fof::Term::Function(function) => match function.as_ref() {
            fof::FunctionTerm::Plain(term) => import_plain_term(term, symbols, variables),
            fof::FunctionTerm::System(term) => import_system_term(term, symbols, variables),
            fof::FunctionTerm::Defined(fof::DefinedTerm::Defined(value)) => match value {
                CommonDefinedTerm::Number(number) => {
                    import_special_constant(&format!("number_{number}"), symbols)
                }
                CommonDefinedTerm::Distinct(value) => {
                    import_special_constant(&format!("object_{}", value.0), symbols)
                }
            },
            fof::FunctionTerm::Defined(fof::DefinedTerm::Atomic(fof::DefinedAtomicTerm(term))) => {
                import_defined_term(term, symbols, variables)
            }
        },
    }
}

fn import_plain_term(
    term: &fof::PlainTerm<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::PlainTerm::Constant(constant) => {
            render_function(atomic_word(&constant.0.0), &[], symbols)
        }
        fof::PlainTerm::Function(functor, arguments) => {
            let arguments = import_arguments(arguments, symbols, variables)?;
            render_function(atomic_word(&functor.0), &arguments, symbols)
        }
    }
}

fn import_system_term(
    term: &fof::SystemTerm<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::SystemTerm::Constant(constant) => {
            render_function(&constant.0.to_string(), &[], symbols)
        }
        fof::SystemTerm::Function(functor, arguments) => {
            let arguments = import_arguments(arguments, symbols, variables)?;
            render_function(&functor.to_string(), &arguments, symbols)
        }
    }
}

fn import_defined_term(
    term: &fof::DefinedPlainTerm<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::DefinedPlainTerm::Constant(constant) => {
            render_function(&constant.0.to_string(), &[], symbols)
        }
        fof::DefinedPlainTerm::Function(functor, arguments) => {
            let arguments = import_arguments(arguments, symbols, variables)?;
            render_function(&functor.to_string(), &arguments, symbols)
        }
    }
}

fn import_arguments(
    arguments: &fof::Arguments<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<Vec<String>, String> {
    arguments
        .0
        .iter()
        .map(|argument| import_term(argument, symbols, variables))
        .collect()
}

fn import_plain_predicate(
    term: &fof::PlainTerm<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::PlainTerm::Constant(constant) => {
            render_predicate(atomic_word(&constant.0.0), &[], symbols)
        }
        fof::PlainTerm::Function(functor, arguments) => {
            let arguments = import_arguments(arguments, symbols, variables)?;
            render_predicate(atomic_word(&functor.0), &arguments, symbols)
        }
    }
}

fn import_system_predicate(
    term: &fof::SystemTerm<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::SystemTerm::Constant(constant) => {
            render_predicate(&constant.0.to_string(), &[], symbols)
        }
        fof::SystemTerm::Function(functor, arguments) => {
            let arguments = import_arguments(arguments, symbols, variables)?;
            render_predicate(&functor.to_string(), &arguments, symbols)
        }
    }
}

fn import_defined_predicate(
    term: &fof::DefinedPlainTerm<'_>,
    symbols: &mut Symbols,
    variables: &mut ImportVariables,
) -> Result<String, String> {
    match term {
        fof::DefinedPlainTerm::Constant(constant) => {
            render_predicate(&constant.0.to_string(), &[], symbols)
        }
        fof::DefinedPlainTerm::Function(functor, arguments) => {
            let arguments = import_arguments(arguments, symbols, variables)?;
            render_predicate(&functor.to_string(), &arguments, symbols)
        }
    }
}

fn import_special_constant(raw: &str, symbols: &mut Symbols) -> Result<String, String> {
    render_function(&format!("tptp_{raw}"), &[], symbols)
}

fn render_function(
    raw: &str,
    arguments: &[String],
    symbols: &mut Symbols,
) -> Result<String, String> {
    let name = symbols.function(raw, arguments.len())?;
    Ok(render_call(&name, arguments))
}

fn render_predicate(
    raw: &str,
    arguments: &[String],
    symbols: &mut Symbols,
) -> Result<String, String> {
    let name = symbols.predicate(raw, arguments.len())?;
    Ok(render_call(&name, arguments))
}

fn render_call(name: &str, arguments: &[String]) -> String {
    if arguments.is_empty() {
        format!("({name})")
    } else {
        format!("({name} {})", arguments.join(" "))
    }
}

fn atomic_word<'a>(word: &'a AtomicWord<'a>) -> &'a str {
    match word {
        AtomicWord::Lower(word) => word.0,
        AtomicWord::SingleQuoted(word) => word.0,
    }
}

#[derive(Default)]
struct Symbols {
    functions: BTreeMap<String, usize>,
    predicates: BTreeMap<String, usize>,
    raw_names: HashMap<String, String>,
}

impl Symbols {
    fn function(&mut self, raw: &str, arity: usize) -> Result<String, String> {
        let name = self.name(raw)?;
        if self.predicates.contains_key(&name) {
            return Err(format!(
                "TPTP symbol {raw} is used as both a function and predicate"
            ));
        }
        register_arity(&mut self.functions, &name, arity, raw)?;
        Ok(name)
    }

    fn predicate(&mut self, raw: &str, arity: usize) -> Result<String, String> {
        let name = self.name(raw)?;
        if self.functions.contains_key(&name) {
            return Err(format!(
                "TPTP symbol {raw} is used as both a function and predicate"
            ));
        }
        register_arity(&mut self.predicates, &name, arity, raw)?;
        Ok(name)
    }

    fn name(&mut self, raw: &str) -> Result<String, String> {
        let name = egglog_identifier(raw);
        if let Some(previous) = self.raw_names.get(&name) {
            if previous != raw {
                return Err(format!(
                    "TPTP symbols {previous} and {raw} map to the same egglog name {name}"
                ));
            }
        } else {
            self.raw_names.insert(name.clone(), raw.to_owned());
        }
        Ok(name)
    }
}

fn register_arity(
    symbols: &mut BTreeMap<String, usize>,
    name: &str,
    arity: usize,
    raw: &str,
) -> Result<(), String> {
    if let Some(previous) = symbols.insert(name.to_owned(), arity)
        && previous != arity
    {
        return Err(format!(
            "TPTP symbol {raw} is used with arities {previous} and {arity}"
        ));
    }
    Ok(())
}

fn egglog_identifier(raw: &str) -> String {
    if !raw.is_empty()
        && raw
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !raw.as_bytes()[0].is_ascii_digit()
    {
        raw.to_owned()
    } else {
        format!(
            "tptp_{}",
            raw.as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    }
}

#[derive(Default)]
struct ImportVariables {
    assigned: HashMap<String, String>,
    used: HashSet<String>,
}

impl ImportVariables {
    fn get(&mut self, tptp_name: &str) -> String {
        if let Some(name) = self.assigned.get(tptp_name) {
            return name.clone();
        }
        let stripped = tptp_name.strip_prefix("V_").unwrap_or(tptp_name);
        let mut base = stripped.to_ascii_lowercase();
        if base.is_empty() || !base.as_bytes()[0].is_ascii_alphabetic() {
            base.insert_str(0, "v_");
        }
        let mut name = base.clone();
        let mut suffix = 2;
        while self.used.contains(&name) {
            name = format!("{base}_{suffix}");
            suffix += 1;
        }
        self.used.insert(name.clone());
        self.assigned.insert(tptp_name.to_owned(), name.clone());
        name
    }
}

struct SignedAtom {
    positive: bool,
    atom: Atom,
}

#[derive(Debug)]
enum Atom {
    Equality(String, String),
    Predicate(String),
}

impl Atom {
    fn as_condition(&self) -> String {
        match self {
            Self::Equality(lhs, rhs) => format!("(= {lhs} {rhs})"),
            Self::Predicate(atom) => atom.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    use egglog::EGraph;

    use super::{cnf_to_egglog, fof_to_egglog};

    #[test]
    fn imports_horn_cnf_as_rewrites_and_rules() {
        let cnf = r#"
            cnf(eq, axiom, (~ p(X) | X != a | f(X) = X)).
            cnf(pred, axiom, (~ p(X) | q(X))).
        "#;
        let egglog = cnf_to_egglog(cnf).unwrap();
        assert_eq!(
            egglog,
            "(sort TptpTerm)\n\
             (constructor a () TptpTerm)\n\
             (constructor f (TptpTerm) TptpTerm)\n\
             (relation p (TptpTerm))\n\
             (relation q (TptpTerm))\n\
             \n\
             (rewrite (f x) x :when ((p x) (= x (a))))\n\
             (rule ((p x)) ((q x)))\n"
        );
        assert_valid_egglog(&egglog);
    }

    #[test]
    fn imports_bitvector_cnf_that_the_egglog_cli_accepts() {
        let egglog = cnf_to_egglog(include_str!("../../benchmarks/bitvec/bitvec.p")).unwrap();
        assert!(egglog.contains("(rewrite (add (zero) n) n)"), "{egglog}");
        assert_egglog_cli_accepts(&egglog);
    }

    #[test]
    fn rejects_non_horn_cnf() {
        let error = cnf_to_egglog("cnf(c, axiom, (p(X) | q(X))).").unwrap_err();
        assert!(error.contains("not Horn"), "{error}");
    }

    #[test]
    fn clausifies_and_skolemizes_fof_with_vampire() {
        let fof = "fof(e, axiom, ! [X] : (? [Y] : (p(X) => f(X) = g(Y)))).";
        let egglog = fof_to_egglog(fof).unwrap();
        assert!(egglog.contains("(constructor sK0 (TptpTerm) TptpTerm)"));
        assert!(
            egglog.contains("(birewrite (f x0) (g (sK0 x0)) :when ((p x0)))"),
            "{egglog}"
        );
        assert_valid_egglog(&egglog);
    }

    fn assert_valid_egglog(source: &str) {
        EGraph::default()
            .parse_and_run_program(None, source)
            .unwrap();
    }

    fn assert_egglog_cli_accepts(source: &str) {
        let mut child = Command::new("egglog")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not run egglog");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        let output = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output.contains("[ERROR]"),
            "egglog rejected imported CNF:\n{output}"
        );
    }
}
