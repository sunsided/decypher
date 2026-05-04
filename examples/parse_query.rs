use cypher::ast::ToCypher;

fn main() {
    let code = "MATCH (n:Person) WHERE n.age > 18 RETURN n.name;";

    // ── CST (lossless rowan tree) ──────────────────────────────────────
    let parse = cypher::cst::parse(code);
    println!("=== CST (Concrete Syntax Tree) ===");
    print_tree(&parse.tree, 0);

    // ── AST ────────────────────────────────────────────────────────────
    match cypher::parse(code) {
        Ok(query) => {
            println!("\n=== AST (Abstract Syntax Tree) ===");
            println!("{:#?}", query);

            println!("\n=== AST round-trip ===");
            println!("{}", query.display());
        }
        Err(err) => eprintln!("ERROR: {}", err),
    }
}

/// Print a rowan syntax node as an indented tree, skipping trivia.
fn print_tree(node: &cypher::syntax::SyntaxNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{:?}", indent, node);

    for child in node.children_with_tokens() {
        if let Some(n) = child.as_node() {
            if !is_trivia(n.kind()) {
                print_tree(n, depth + 1);
            }
        } else if let Some(t) = child.as_token() {
            if !is_trivia(t.kind()) {
                println!("{}  {:?}", indent, t);
            }
        }
    }
}

fn is_trivia(kind: cypher::syntax::SyntaxKind) -> bool {
    matches!(
        kind,
        cypher::syntax::SyntaxKind::WHITESPACE | cypher::syntax::SyntaxKind::COMMENT
    )
}
