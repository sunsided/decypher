// F01. Nested-looking comments, depending on your lexer policy
RETURN /* outer /* inner */ still comment? */ 1 AS x;
