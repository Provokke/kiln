//! Property-based tests:
//! * random ASTs survive print -> parse unchanged (formatter round trip);
//! * the lexer/parser never panic on arbitrary input (parser fuzzing);
//! * randomly generated numeric programs print the same output in
//!   interpreter, JIT and GC-stress modes (differential across tiers).

use kiln::ast::*;
use kiln::lexer::Span;
use kiln::vm::Options;
use proptest::prelude::*;

const S: Span = Span { start: 0, end: 0 };

fn ident() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["a", "b", "foo", "x1", "$y", "_z", "été"]).prop_map(String::from)
}

fn string_lit() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec!['a', 'Z', ' ', '"', '\'', '\\', '\n', '\t', '`', '$', '{', 'é', '😀', '0']),
        0..6,
    )
    .prop_map(|v| v.into_iter().collect())
}

fn number() -> impl Strategy<Value = f64> {
    prop_oneof![
        (0u32..100000).prop_map(|n| n as f64),
        (0.0f64..1e6),
        Just(1e21),
        Just(1.5e-7),
        Just(0.1)
    ]
}

fn expr() -> BoxedStrategy<Expr> {
    let leaf = prop_oneof![
        number().prop_map(|n| Expr::Num(n, S)),
        string_lit().prop_map(|s| Expr::Str(s, S)),
        ident().prop_map(|n| Expr::Ident(n, S)),
        any::<bool>().prop_map(|b| Expr::Bool(b, S)),
        Just(Expr::Null(S)),
        Just(Expr::This(S)),
    ];
    leaf.prop_recursive(5, 48, 4, |inner| {
        let bin = prop::sample::select(vec![
            "+",
            "-",
            "*",
            "/",
            "%",
            "**",
            "<",
            "<=",
            ">",
            ">=",
            "==",
            "!=",
            "===",
            "!==",
            "&",
            "|",
            "^",
            "<<",
            ">>",
            ">>>",
            "instanceof",
            "in",
        ]);
        prop_oneof![
            (bin, inner.clone(), inner.clone()).prop_map(|(op, l, r)| Expr::Binary(op, Box::new(l), Box::new(r), S)),
            (prop::sample::select(vec!["&&", "||", "??"]), inner.clone(), inner.clone()).prop_map(|(op, l, r)| Expr::Logical(
                op,
                Box::new(l),
                Box::new(r),
                S
            )),
            (prop::sample::select(vec!["-", "+", "!", "~", "typeof", "void"]), inner.clone()).prop_map(|(op, e)| Expr::Unary(
                op,
                Box::new(e),
                S
            )),
            (inner.clone(), inner.clone(), inner.clone()).prop_map(|(a, b, c)| Expr::Cond(Box::new(a), Box::new(b), Box::new(c), S)),
            (inner.clone(), prop::collection::vec(inner.clone(), 0..3)).prop_map(|(c, a)| Expr::Call(Box::new(c), a, S)),
            (ident(), prop::collection::vec(inner.clone(), 0..2)).prop_map(|(c, a)| Expr::New(Box::new(Expr::Ident(c, S)), a, S)),
            (inner.clone(), ident()).prop_map(|(o, n)| Expr::Member(Box::new(o), n, S)),
            (inner.clone(), inner.clone()).prop_map(|(o, i)| Expr::Index(Box::new(o), Box::new(i), S)),
            prop::collection::vec(inner.clone(), 0..4).prop_map(|v| Expr::Array(v, S)),
            prop::collection::vec((ident(), inner.clone()), 0..3)
                .prop_map(|v| Expr::Object(v.into_iter().map(|(k, e)| Prop::KeyValue(PropKey::Ident(k), e)).collect(), S)),
            (ident(), prop::sample::select(vec!["=", "+=", "*=", "**=", "??="]), inner.clone()).prop_map(|(n, op, v)| Expr::Assign(
                op,
                Box::new(Expr::Ident(n, S)),
                Box::new(v),
                S
            )),
            (prop::collection::vec(ident(), 0..3), inner.clone()).prop_map(|(ps, b)| {
                let mut ps = ps;
                ps.dedup();
                ps.sort();
                ps.dedup();
                Expr::Function(Box::new(Function {
                    name: None,
                    params: ps,
                    body: vec![Stmt::Return(Some(b), S)],
                    is_arrow: true,
                    expr_body: true,
                    span: S,
                }))
            }),
            (string_lit(), inner.clone(), string_lit()).prop_map(|(a, e, b)| Expr::Template(vec![a, b], vec![e], S)),
            (ident(), any::<bool>(), prop::sample::select(vec!["++", "--"])).prop_map(|(n, prefix, op)| Expr::Update {
                op,
                prefix,
                target: Box::new(Expr::Ident(n, S)),
                span: S
            }),
            prop::collection::vec(inner, 2..4).prop_map(|v| Expr::Seq(v, S)),
        ]
    })
    .boxed()
}

fn stmt() -> impl Strategy<Value = Stmt> {
    let e = expr();
    prop_oneof![
        e.clone().prop_map(|e| Stmt::Expr(e, S)),
        (ident(), e.clone()).prop_map(|(n, e)| Stmt::Var(VarKind::Let, vec![(n, Some(e))], S)),
        (e.clone(), e.clone(), prop::option::of(e.clone())).prop_map(|(c, t, f)| Stmt::If(
            c,
            Box::new(Stmt::Block(vec![Stmt::Expr(t, S)], S)),
            f.map(|f| Box::new(Stmt::Block(vec![Stmt::Expr(f, S)], S))),
            S
        )),
        (e.clone(), e.clone()).prop_map(|(c, b)| Stmt::While(c, Box::new(Stmt::Block(vec![Stmt::Expr(b, S), Stmt::Break(S)], S)), S)),
        (ident(), e.clone()).prop_map(|(n, b)| Stmt::Function(Function {
            name: Some(n),
            params: vec!["p".into()],
            body: vec![Stmt::Return(Some(b), S)],
            is_arrow: false,
            expr_body: false,
            span: S
        })),
        (e.clone(), e).prop_map(|(a, b)| Stmt::Try(vec![Stmt::Throw(a, S)], Some((Some("err".into()), vec![Stmt::Expr(b, S)])), None, S)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, ..ProptestConfig::default() })]

    #[test]
    fn print_parse_roundtrip(body in prop::collection::vec(stmt(), 1..5)) {
        let prog = Program { body };
        let printed = kiln::printer::print_program(&prog);
        let reparsed = kiln::parser::parse(&printed).map_err(|e| TestCaseError::fail(format!("{}\n{printed}", e.render(&printed, "gen"))))?;
        prop_assert_eq!(&reparsed, &prog, "printed:\n{}", printed);
        prop_assert_eq!(kiln::printer::print_program(&reparsed), printed);
    }

    #[test]
    fn parser_never_panics_on_garbage(src in "[ -~\\n]{0,80}") {
        let _ = kiln::parser::parse(&src);
    }

    #[test]
    fn parser_never_panics_on_token_soup(toks in prop::collection::vec(prop::sample::select(vec![
        "(", ")", "{", "}", "[", "]", "=>", "function", "class", "let", "x", "1", "'s'", "`a${", "}`", ",", ";", "+", "=", "?", ":", ".", "new", "return", "if", "else", "for", "of",
    ]), 0..40)) {
        let src = toks.join(" ");
        let _ = kiln::parser::parse(&src);
    }
}

// ------------------------------------------------------------------ random numeric programs

fn num_expr(vars: &'static [&'static str]) -> BoxedStrategy<String> {
    let leaf = prop_oneof![
        (0i32..20).prop_map(|n| n.to_string()),
        Just("2147483647".to_string()),
        Just("0.5".to_string()),
        prop::sample::select(vars).prop_map(String::from),
    ];
    leaf.prop_recursive(3, 12, 2, |inner| {
        (inner.clone(), prop::sample::select(vec!["+", "-", "*", "%", "/"]), inner).prop_map(|(a, op, b)| format!("({a} {op} {b})"))
    })
    .boxed()
}

fn program() -> impl Strategy<Value = String> {
    const VARS: &[&str] = &["a", "b", "x", "y"];
    (
        num_expr(VARS),
        num_expr(VARS),
        num_expr(VARS),
        prop::sample::select(vec!["<", "<=", ">", "!==", "==="]),
        0i32..15,
        prop::collection::vec((-50i32..50, -5i32..9), 1..5),
    )
        .prop_map(|(e1, e2, e3, cmp, n, args)| {
            let mut s = format!(
                "function f(a, b) {{ let x = 1; let y = 0; for (let i = 0; i < {n}; i++) {{ x = {e1}; if (x {cmp} y) {{ y = {e2}; }} else {{ y = y + 1; }} if (y > 1000000) {{ y = y % 997; }} }} return {e3}; }}\n"
            );
            for (a, b) in args {
                s += &format!("console.log(f({a}, {b}));\n");
            }
            s += "let t = 0; for (let k = 0; k < 20; k++) t = t + f(k, 3); console.log(t);\n";
            s
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 150, ..ProptestConfig::default() })]

    #[test]
    fn random_programs_agree_across_tiers(src in program()) {
        let s1 = src.clone();
        let interp = kiln::with_big_stack(move || kiln::run_source(&s1, Options { jit: false, ..Options::default() }).output);
        let s2 = src.clone();
        let jit = kiln::with_big_stack(move || kiln::run_source(&s2, Options { jit: true, jit_threshold: 0, ..Options::default() }).output);
        let s3 = src.clone();
        let stress = kiln::with_big_stack(move || kiln::run_source(&s3, Options { jit: true, jit_threshold: 1, gc_stress: true, ..Options::default() }).output);
        prop_assert!(interp.lines().count() >= 2, "program produced no output:\n{}", src);
        prop_assert_eq!(&interp, &jit, "program:\n{}", src);
        prop_assert_eq!(&interp, &stress, "program:\n{}", src);
    }
}
