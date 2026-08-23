use egglog::{
    EGraph,
    ast::{Command, Expr, Literal},
};
use eq_prog_opt::{Decl, Term, parse};
use std::collections::HashSet;

/// Convert an eq-prog-opt benchmark into egglog syntax.
pub fn eq_prog_opt_to_egglog(source: &str) -> Result<String, String> {
    let declarations = parse::parse_decls(source)?;
    let constants = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Decl::Function(function) if function.args.is_empty() => Some(function.name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let mut output = Vec::new();

    for declaration in declarations {
        match declaration {
            Decl::Sort(sort) => output.push(format!("(sort {})", sort.name)),
            Decl::Function(function) => output.push(format!(
                "(constructor {} ({}) {})",
                function.name,
                function.args.join(" "),
                function.ret
            )),
            Decl::Rewrite(rewrite) => output.push(format!(
                "(rewrite {} {})",
                eq_term_to_egglog(&rewrite.lhs, &constants),
                eq_term_to_egglog(&rewrite.rhs, &constants)
            )),
            Decl::Optimize(optimize) => output.push(format!(
                "(extract {})",
                eq_term_to_egglog(&optimize.term, &constants)
            )),
        }
    }

    if output.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", output.join("\n")))
    }
}

fn eq_term_to_egglog(term: &Term, constants: &HashSet<String>) -> String {
    match term {
        Term::Var(name) if constants.contains(name) => format!("({name})"),
        Term::Var(name) => name.clone(),
        Term::Num(number) => number.to_string(),
        Term::Call(function, arguments) => {
            let arguments = arguments
                .iter()
                .map(|argument| eq_term_to_egglog(argument, constants))
                .collect::<Vec<_>>();
            if arguments.is_empty() {
                format!("({function})")
            } else {
                format!("({function} {})", arguments.join(" "))
            }
        }
    }
}

/// Convert the eq-prog-opt-compatible fragment of egglog into its benchmark
/// syntax.
pub fn egglog_to_eq_prog_opt(source: &str, filename: Option<String>) -> Result<String, String> {
    let commands = EGraph::default()
        .parse_program(filename, source)
        .map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    let mut rewrite_index = 1;

    for command in commands {
        match command {
            Command::Sort {
                name,
                presort_and_args: None,
                ..
            } => output.push(format!("(sort {name})")),
            Command::Sort { name, .. } => {
                return Err(format!(
                    "container sort {name} is not supported by eq-prog-opt"
                ));
            }
            Command::Datatype { name, variants, .. } => {
                output.push(format!("(sort {name})"));
                for variant in variants {
                    output.push(format!(
                        "(function {} ({}) {name})",
                        variant.name,
                        variant.types.join(" ")
                    ));
                }
            }
            Command::Constructor { name, schema, .. } => output.push(format!(
                "(function {name} ({}) {})",
                schema.input.join(" "),
                schema.output
            )),
            Command::Function { name, schema, .. } => output.push(format!(
                "(function {name} ({}) {})",
                schema.input.join(" "),
                schema.output
            )),
            Command::Rewrite(_, rewrite, _) => {
                ensure_unconditional(&rewrite)?;
                output.push(format!(
                    "(rewrite rewrite_{rewrite_index} {} {})",
                    expr_to_eq_prog_opt(&rewrite.lhs)?,
                    expr_to_eq_prog_opt(&rewrite.rhs)?
                ));
                rewrite_index += 1;
            }
            Command::BiRewrite(_, rewrite) => {
                ensure_unconditional(&rewrite)?;
                let lhs = expr_to_eq_prog_opt(&rewrite.lhs)?;
                let rhs = expr_to_eq_prog_opt(&rewrite.rhs)?;
                output.push(format!(
                    "(rewrite rewrite_{rewrite_index}_forward {lhs} {rhs})"
                ));
                output.push(format!(
                    "(rewrite rewrite_{rewrite_index}_backward {rhs} {lhs})"
                ));
                rewrite_index += 1;
            }
            Command::Extract(_, expression, _) => {
                output.push(format!("(optimize {})", expr_to_eq_prog_opt(&expression)?));
            }
            Command::Rule { .. } => {
                return Err("general egglog rules are not supported by eq-prog-opt".into());
            }
            Command::Relation { name, .. } => {
                return Err(format!("relation {name} is not supported by eq-prog-opt"));
            }
            _ => {}
        }
    }

    if output.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", output.join("\n")))
    }
}

fn ensure_unconditional(rewrite: &egglog::ast::Rewrite) -> Result<(), String> {
    if rewrite.conditions.is_empty() {
        Ok(())
    } else {
        Err("conditional rewrites are not supported by eq-prog-opt".into())
    }
}

fn expr_to_eq_prog_opt(expression: &Expr) -> Result<String, String> {
    match expression {
        Expr::Var(_, variable) => Ok(variable.clone()),
        Expr::Call(_, function, arguments) => {
            let arguments = arguments
                .iter()
                .map(expr_to_eq_prog_opt)
                .collect::<Result<Vec<_>, _>>()?;
            if arguments.is_empty() {
                Ok(format!("({function})"))
            } else {
                Ok(format!("({function} {})", arguments.join(" ")))
            }
        }
        Expr::Lit(_, Literal::Int(value)) => Ok(value.to_string()),
        Expr::Lit(_, Literal::String(value)) => Ok(format!("\"{}\"", value.replace('"', "\\\""))),
        Expr::Lit(_, literal) => Err(format!(
            "egglog literal {literal} is not supported by eq-prog-opt"
        )),
    }
}

#[cfg(test)]
mod tests {
    use egglog::EGraph;
    use tptp::TPTPIterator;

    use super::{egglog_to_eq_prog_opt, eq_prog_opt_to_egglog};
    use crate::{Format, convert};

    const EQ_PROG_OPT: &str = r#"
        (sort Math)
        (function Zero () Math)
        (function Add (Math Math) Math)
        (rewrite add-zero (Add Zero x) x)
        (optimize (Add Zero Zero))
    "#;

    #[test]
    fn imports_eq_prog_opt_through_its_library_parser() {
        let egglog = eq_prog_opt_to_egglog(EQ_PROG_OPT).unwrap();
        assert_eq!(
            egglog,
            "(sort Math)\n\
             (constructor Zero () Math)\n\
             (constructor Add (Math Math) Math)\n\
             (rewrite (Add (Zero) x) x)\n\
             (extract (Add (Zero) (Zero)))\n"
        );
        EGraph::default()
            .parse_program(None, &egglog)
            .expect("generated egglog should parse");
    }

    #[test]
    fn exports_egglog_and_expands_birewrites() {
        let egglog = r#"
            (datatype Math (Zero) (Add Math Math))
            (rewrite (Add (Zero) x) x)
            (birewrite (Add x y) (Add y x))
            (extract (Add (Zero) (Zero)))
        "#;
        let output = egglog_to_eq_prog_opt(egglog, None).unwrap();
        assert!(output.contains("(sort Math)"));
        assert!(output.contains("(function Add (Math Math) Math)"));
        assert!(output.contains("(rewrite rewrite_1 (Add (Zero) x) x)"));
        assert!(output.contains("rewrite_2_forward"));
        assert!(output.contains("rewrite_2_backward"));
        parse::parse_decls(&output).expect("generated eq-prog-opt should parse");
    }

    #[test]
    fn rejects_conditional_rewrites() {
        let source = "(rewrite (f x) x :when ((p x)))";
        let error = egglog_to_eq_prog_opt(source, None).unwrap_err();
        assert!(error.contains("conditional rewrites"), "{error}");
    }

    #[test]
    fn routes_eq_prog_opt_through_tptp_formats() {
        let cnf = convert(EQ_PROG_OPT, None, Format::EqProgOpt, Format::Cnf).unwrap();
        assert!(cnf.contains("'Add'('Zero', V_x) = V_x"), "{cnf}");
        assert!(
            TPTPIterator::<()>::new(cnf.as_bytes()).all(|formula| formula.is_ok()),
            "{cnf}"
        );

        let round_trip = convert(&cnf, None, Format::Cnf, Format::EqProgOpt).unwrap();
        parse::parse_decls(&round_trip).expect("round-tripped eq-prog-opt should parse");
    }

    mod parse {
        pub use eq_prog_opt::parse::parse_decls;
    }
}
