// F01. Nested-looking comments, depending on your lexer policy
RETURN /* outer /* inner */ still comment? */ 1 AS x;

// F02. Malformed rich label expression
MATCH (n:(Person|)) RETURN n;

// F03. Invalid quantified path bound list
MATCH p = ((a)-[:R]->(b)){,} RETURN p;

// F04. Empty COUNT subquery body
RETURN COUNT { };
