// E01. Comments everywhere
MATCH /* block comment */ (n) // line comment
RETURN n;

// E02. Unicode strings
RETURN 'Größe ÄÖÜ ñ 漢字' AS text;

// E03. Escapes
RETURN 'line\nbreak\tTabbed\\slash\'quote' AS escaped;

// E04. Integer formats
RETURN 0 AS zero, 123 AS decimal, -456 AS negative;

// E05. Scientific notation
RETURN 1e3 AS a, 1.2e-3 AS b, -4.5E+6 AS c;

// E06. Weird whitespace
MATCH
  (n)
WHERE
  n.name
  =
  'Alice'
RETURN
  n;

// E07. Multiple statements in one script
CREATE (:Tmp {x: 1});
MATCH (n:Tmp) RETURN n;
MATCH (n:Tmp) DETACH DELETE n;

// E08. Reserved-looking escaped names
RETURN 1 AS `MATCH`, 2 AS `RETURN`, 3 AS `WHERE`;

// E09. Chained property/index access
WITH {a: [{b: 1}, {b: 2}]} AS m
RETURN m.a[0].b AS value;

// E10. Deep expression precedence
RETURN NOT 1 + 2 * 3 < 10 AND false OR true XOR false AS result;

// E11. Parenthesized expression
RETURN (((1 + 2) * (3 + 4))) AS result;

// E12. Empty lists and maps
RETURN [] AS emptyList, {} AS emptyMap;

// E13. Null arithmetic
RETURN null + 1 AS value;

// E14. Duplicate aliases, depending on semantic layer
RETURN 1 AS x, 2 AS x;
