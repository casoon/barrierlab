//! Parses a real multi-directive CSP and prints its structured
//! directives. Run with `cargo run --example parse`.

use csp_parse::{DirectiveValue, parse_policy_list};

fn main() {
    let policy = "default-src 'self'; \
                  script-src 'self' 'nonce-2726c7f26c' https://cdn.example.com; \
                  img-src 'self' data:; \
                  frame-ancestors 'none'; \
                  upgrade-insecure-requests; \
                  report-to csp-endpoint";

    println!("Parsing: {policy}\n");

    let policy_list = parse_policy_list(policy);
    for policy in &policy_list.policies {
        for directive in &policy.directives {
            println!("{}:", directive.name);
            match directive.value() {
                DirectiveValue::SourceList(list) => println!("  source list: {list:?}"),
                DirectiveValue::AncestorSourceList(list) => {
                    println!("  ancestor source list: {list:?}");
                }
                DirectiveValue::Boolean => println!("  (no value)"),
                DirectiveValue::Token(token) => println!("  token: {token:?}"),
                other => println!("  {other:?}"),
            }
        }
    }
}
