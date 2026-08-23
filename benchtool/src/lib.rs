use std::collections::{HashMap, HashSet};

use clap::ValueEnum;
use egglog::{
    EGraph,
    ast::{Command, Expr, Fact, Literal, Rewrite},
};

mod eqprog;
mod import;
mod smtlib;
pub use eqprog::{egglog_to_eq_prog_opt, eq_prog_opt_to_egglog};
pub use import::{cnf_to_egglog, fof_to_egglog};
pub use smtlib::{egglog_to_smtlib, smtlib_to_egglog, smtlib_to_egglog_untyped};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Format {
    Egglog,
    EqProgOpt,
    Cnf,
    Fof,
    Smtlib,
}

/// Convert between the currently supported benchmark formats.
pub fn convert(
    source: &str,
    filename: Option<String>,
    from: Format,
    to: Format,
) -> Result<String, String> {
    match (from, to) {
        (from, to) if from == to => Ok(source.to_owned()),
        (Format::Egglog, Format::Cnf) => {
            egglog_to_tptp_with_format(source, filename, TptpFormat::Cnf)
        }
        (Format::Egglog, Format::Fof) => {
            egglog_to_tptp_with_format(source, filename, TptpFormat::Fof)
        }
        (Format::Egglog, Format::Smtlib) => egglog_to_smtlib(source, filename),
        (Format::Cnf, Format::Egglog) => cnf_to_egglog(source),
        (Format::Fof, Format::Egglog) => fof_to_egglog(source),
        (Format::Smtlib, Format::Egglog) => smtlib_to_egglog(source),
        (Format::EqProgOpt, Format::Egglog) => eq_prog_opt_to_egglog(source),
        (Format::Egglog, Format::EqProgOpt) => egglog_to_eq_prog_opt(source, filename),
        (Format::EqProgOpt, Format::Cnf | Format::Fof) => {
            let egglog = eq_prog_opt_to_egglog(source)?;
            let format = if to == Format::Cnf {
                TptpFormat::Cnf
            } else {
                TptpFormat::Fof
            };
            egglog_to_tptp_with_format(&egglog, filename, format)
        }
        (Format::EqProgOpt, Format::Smtlib) => {
            let egglog = eq_prog_opt_to_egglog(source)?;
            egglog_to_smtlib(&egglog, filename)
        }
        (Format::Cnf, Format::EqProgOpt) => {
            let egglog = cnf_to_egglog(source)?;
            egglog_to_eq_prog_opt(&egglog, filename)
        }
        (Format::Fof, Format::EqProgOpt) => {
            let egglog = fof_to_egglog(source)?;
            egglog_to_eq_prog_opt(&egglog, filename)
        }
        (Format::Cnf, Format::Smtlib) => {
            let egglog = cnf_to_egglog(source)?;
            egglog_to_smtlib(&egglog, filename)
        }
        (Format::Fof, Format::Smtlib) => {
            let egglog = fof_to_egglog(source)?;
            egglog_to_smtlib(&egglog, filename)
        }
        (Format::Smtlib, Format::Cnf | Format::Fof) => {
            let egglog = smtlib_to_egglog(source)?;
            let format = if to == Format::Cnf {
                TptpFormat::Cnf
            } else {
                TptpFormat::Fof
            };
            egglog_to_tptp_with_format(&egglog, filename, format)
        }
        (Format::Smtlib, Format::EqProgOpt) => {
            let egglog = smtlib_to_egglog(source)?;
            egglog_to_eq_prog_opt(&egglog, filename)
        }
        _ => Err(format!(
            "conversion from {from:?} to {to:?} is not supported yet"
        )),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum TptpFormat {
    Cnf,
    Fof,
}

/// Convert the `rewrite` and `birewrite` commands in an egglog program to
/// equality clauses in TPTP CNF format.
pub fn egglog_to_tptp(source: &str, filename: Option<String>) -> Result<String, String> {
    egglog_to_tptp_with_format(source, filename, TptpFormat::Cnf)
}

/// Convert the `rewrite` and `birewrite` commands in an egglog program to
/// TPTP CNF clauses or FOF formulas.
pub fn egglog_to_tptp_with_format(
    source: &str,
    filename: Option<String>,
    format: TptpFormat,
) -> Result<String, String> {
    let commands = EGraph::default()
        .parse_program(filename, source)
        .map_err(|error| error.to_string())?;
    let mut clauses = Vec::new();

    for command in commands {
        let (kind, rewrite) = match command {
            Command::Rewrite(_, rewrite, _) => ("rewrite", rewrite),
            Command::BiRewrite(_, rewrite) => ("birewrite", rewrite),
            _ => continue,
        };

        clauses.push(rewrite_to_tptp(kind, clauses.len() + 1, &rewrite, format)?);
    }

    if clauses.is_empty() {
        return Ok(String::new());
    }

    Ok(format!("{}\n", clauses.join("\n")))
}

fn rewrite_to_tptp(
    kind: &str,
    index: usize,
    rewrite: &Rewrite,
    format: TptpFormat,
) -> Result<String, String> {
    let mut variables = VariableNames::default();
    match format {
        TptpFormat::Cnf => {
            let mut literals = rewrite
                .conditions
                .iter()
                .map(|condition| negative_condition_to_tptp(condition, &mut variables))
                .collect::<Result<Vec<_>, _>>()?;
            let lhs = term_to_tptp(&rewrite.lhs, &mut variables)?;
            let rhs = term_to_tptp(&rewrite.rhs, &mut variables)?;
            literals.push(format!("{lhs} = {rhs}"));
            Ok(format!(
                "cnf({kind}_{index}, axiom, ({})).",
                literals.join(" | ")
            ))
        }
        TptpFormat::Fof => {
            let conditions = rewrite
                .conditions
                .iter()
                .map(|condition| positive_condition_to_tptp(condition, &mut variables))
                .collect::<Result<Vec<_>, _>>()?;
            let lhs = term_to_tptp(&rewrite.lhs, &mut variables)?;
            let rhs = term_to_tptp(&rewrite.rhs, &mut variables)?;
            let equality = format!("{lhs} = {rhs}");
            let body = if conditions.is_empty() {
                equality
            } else {
                format!("({}) => ({equality})", conditions.join(" & "))
            };
            let names = variables.sorted_names();
            let formula = if names.is_empty() {
                format!("({body})")
            } else {
                format!("(! [{}] : ({body}))", names.join(", "))
            };
            Ok(format!("fof({kind}_{index}, axiom, {formula})."))
        }
    }
}

fn positive_condition_to_tptp(
    condition: &Fact,
    variables: &mut VariableNames,
) -> Result<String, String> {
    match condition {
        Fact::Eq(_, lhs, rhs) => Ok(format!(
            "{} = {}",
            term_to_tptp(lhs, variables)?,
            term_to_tptp(rhs, variables)?
        )),
        Fact::Fact(atom @ Expr::Call(..)) => term_to_tptp(atom, variables),
        Fact::Fact(_) => Err("a rewrite condition must be an equality or predicate atom".into()),
    }
}

fn negative_condition_to_tptp(
    condition: &Fact,
    variables: &mut VariableNames,
) -> Result<String, String> {
    match condition {
        Fact::Eq(_, lhs, rhs) => Ok(format!(
            "{} != {}",
            term_to_tptp(lhs, variables)?,
            term_to_tptp(rhs, variables)?
        )),
        Fact::Fact(atom @ Expr::Call(..)) => Ok(format!("~ {}", term_to_tptp(atom, variables)?)),
        Fact::Fact(_) => Err("a rewrite condition must be an equality or predicate atom".into()),
    }
}

fn term_to_tptp(expr: &Expr, variables: &mut VariableNames) -> Result<String, String> {
    match expr {
        Expr::Var(_, name) => Ok(variables.get(name)),
        Expr::Call(_, function, arguments) => {
            let function = quote_atom(function);
            if arguments.is_empty() {
                Ok(function)
            } else {
                let arguments = arguments
                    .iter()
                    .map(|argument| term_to_tptp(argument, variables))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{function}({})", arguments.join(", ")))
            }
        }
        Expr::Lit(_, literal) => literal_to_tptp(literal),
    }
}

fn literal_to_tptp(literal: &Literal) -> Result<String, String> {
    match literal {
        Literal::Int(value) => Ok(value.to_string()),
        Literal::Float(value) => {
            let value = value.into_inner();
            if !value.is_finite() {
                return Err(format!(
                    "non-finite float literal is not supported: {value}"
                ));
            }
            let mut rendered = value.to_string();
            if !rendered.contains(['.', 'e', 'E']) {
                rendered.push_str(".0");
            }
            Ok(rendered)
        }
        Literal::String(value) => Ok(format!(
            "\"{}\"",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        )),
        Literal::Bool(value) => Ok(quote_atom(if *value { "true" } else { "false" })),
        Literal::Unit => Ok(quote_atom("unit")),
    }
}

fn quote_atom(name: &str) -> String {
    format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"))
}

#[derive(Default)]
struct VariableNames {
    assigned: HashMap<String, String>,
    used: HashSet<String>,
}

impl VariableNames {
    fn get(&mut self, egglog_name: &str) -> String {
        if let Some(name) = self.assigned.get(egglog_name) {
            return name.clone();
        }

        let cleaned: String = egglog_name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '_' {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        let base = format!("V_{cleaned}");
        let mut name = base.clone();
        let mut suffix = 2;
        while self.used.contains(&name) {
            name = format!("{base}_{suffix}");
            suffix += 1;
        }

        self.used.insert(name.clone());
        self.assigned.insert(egglog_name.to_owned(), name.clone());
        name
    }

    fn sorted_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.assigned.values().cloned().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    use super::{TptpFormat, egglog_to_tptp, egglog_to_tptp_with_format};
    use tptp::TPTPIterator;

    const EQUATIONS: &str = r#"
        (datatype Expr (Zero) (Add Expr Expr))
        (relation Ready (Expr))
        (rewrite (Add (Zero) x) x)
        (birewrite (Add x y) (Add y x))
        (rewrite (Add x (Zero)) x :when ((Ready x) (= x (Zero))))
        (run 3)
    "#;

    #[test]
    fn converts_rewrites_and_ignores_other_commands() {
        let output = egglog_to_tptp(EQUATIONS, None).unwrap();
        assert_eq!(
            output,
            "cnf(rewrite_1, axiom, ('Add'('Zero', V_x) = V_x)).\n\
             cnf(birewrite_2, axiom, ('Add'(V_x, V_y) = 'Add'(V_y, V_x))).\n\
             cnf(rewrite_3, axiom, (~ 'Ready'(V_x) | V_x != 'Zero' | 'Add'(V_x, 'Zero') = V_x)).\n"
        );
        assert_valid_tptp(&output, 3);
    }

    #[test]
    fn eprover_ho_and_vampire_accept_generated_tptp() {
        let output = egglog_to_tptp(EQUATIONS, None).unwrap();
        assert_accepts("eprover-ho", &["--syntax-only", "-"], &output);
        assert_accepts(
            "vampire",
            &["--mode", "clausify", "--input_syntax", "tptp"],
            &output,
        );
    }

    #[test]
    fn reports_parse_errors() {
        let error = egglog_to_tptp("(rewrite (F x)", Some("bad.egg".into())).unwrap_err();
        assert!(error.contains("bad.egg"), "{error}");
    }

    #[test]
    fn converts_equality_and_atom_conditions_to_negative_literals() {
        let source = "(rewrite (F x) x :when ((= x y) (P x)))";
        let output = egglog_to_tptp(source, None).unwrap();
        assert_eq!(
            output,
            "cnf(rewrite_1, axiom, (V_x != V_y | ~ 'P'(V_x) | 'F'(V_x) = V_x)).\n"
        );
        assert_valid_tptp(&output, 1);
    }

    #[test]
    fn converts_conditions_to_fof_implications() {
        let source = "(rewrite (F x) x :when ((= x y) (P x)))";
        let output = egglog_to_tptp_with_format(source, None, TptpFormat::Fof).unwrap();
        assert_eq!(
            output,
            "fof(rewrite_1, axiom, (! [V_x, V_y] : ((V_x = V_y & 'P'(V_x)) => ('F'(V_x) = V_x)))).\n"
        );
        assert_valid_tptp(&output, 1);
        assert_provers_accept(&output);
    }

    fn assert_valid_tptp(output: &str, expected_formulas: usize) {
        let mut parser = TPTPIterator::<()>::new(output.as_bytes());
        let formulas = parser.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(formulas.len(), expected_formulas);
        assert!(parser.remaining.is_empty());
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

    fn assert_provers_accept(input: &str) {
        assert_accepts("eprover-ho", &["--syntax-only", "-"], input);
        assert_accepts(
            "vampire",
            &["--mode", "clausify", "--input_syntax", "tptp"],
            input,
        );
    }
}
