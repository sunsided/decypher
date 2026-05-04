use cypher::parse;

fn main() {
    let code = "MATCH (n:Person) WHERE n.age > 18 RETURN n.name;";
    match parse(code) {
        Ok(query) => println!("{:#?}", query),
        Err(err) => eprintln!("ERROR: {}", err),
    }
}
