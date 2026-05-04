// 01. Empty-ish return, literals, aliases
RETURN
  1 AS intValue,
  1.25 AS floatValue,
  true AS boolValue,
  false AS otherBool,
  null AS nil,
  'hello' AS singleQuoted,
  "world" AS doubleQuoted;

// 02. Parameters
MATCH (n:Person {id: $id})
WHERE n.name = $name AND n.age >= $minAge
RETURN n;

// 03. Escaped identifiers
MATCH (`weird variable`:Person)
RETURN `weird variable`.`strange property` AS value;

// 04. Node labels and label conjunction/disjunction-like label expressions
MATCH (n:Person:Employee)
RETURN n;

// 05. Relationship directions and variable relationship
MATCH (a)-[r:KNOWS]->(b)<-[:LIKES]-(c)
RETURN a, r, b, c;

// 06. Undirected relationship
MATCH (a)-[r:RELATED]-(b)
RETURN a, r, b;

// 07. Relationship type alternatives
MATCH (a)-[r:KNOWS|LIKES|HATES]->(b)
RETURN r;

// 08. Variable-length relationship, classic syntax
MATCH p = (a)-[:KNOWS*1..5]->(b)
RETURN p, length(p);

// 09. Zero-or-more variable length
MATCH p = (a)-[:PARENT_OF*0..]->(b)
RETURN p;

// 10. Exact variable length
MATCH p = (a)-[:NEXT*3]->(b)
RETURN p;

// 11. Multiple disconnected patterns in one MATCH
MATCH (a:Person), (b:Company)
RETURN a, b;

// 12. Path variable
MATCH path = (:Person)-[:WORKS_AT]->(:Company)
RETURN nodes(path), relationships(path);

// 13. OPTIONAL MATCH
MATCH (p:Person)
OPTIONAL MATCH (p)-[:WORKS_AT]->(c:Company)
RETURN p.name, c.name;

// 14. WHERE with boolean operators
MATCH (p:Person)
WHERE
  (p.age > 18 AND p.age < 65)
  OR p.name STARTS WITH 'A'
  XOR p.name ENDS WITH 'z'
RETURN p;

// 15. WHERE with null checks
MATCH (p:Person)
WHERE p.email IS NULL OR p.phone IS NOT NULL
RETURN p;

// 16. WHERE with string operators and regex
MATCH (p:Person)
WHERE p.name CONTAINS 'ann'
  AND p.name =~ '(?i).*ann.*'
RETURN p;

// 17. WHERE with IN and list literal
MATCH (p:Person)
WHERE p.status IN ['active', 'pending', 'blocked']
RETURN p;

// 18. Comparisons
MATCH (p:Person)
WHERE p.age = 42
  OR p.age <> 13
  OR p.score <= 100
  OR p.score >= 0
RETURN p;

// 19. Pattern predicate with EXISTS subquery
MATCH (p:Person)
WHERE EXISTS {
  MATCH (p)-[:WORKS_AT]->(:Company {name: 'Neo4j'})
}
RETURN p;

// 20. Property existence function-style predicate avoided; use IS NOT NULL
MATCH (p:Person)
WHERE p.name IS NOT NULL
RETURN p;

// 21. CREATE nodes and relationship
CREATE (a:Person {name: 'Alice', age: 30})
CREATE (b:Person {name: 'Bob'})
CREATE (a)-[:KNOWS {since: date('2020-01-01')}]->(b)
RETURN a, b;

// 22. CREATE pattern in one clause
CREATE (:Person {name: 'Carol'})-[:LIKES]->(:Thing {name: 'Cypher'})
RETURN 1;

// 23. MERGE node with ON CREATE / ON MATCH
MERGE (p:Person {id: $id})
ON CREATE SET p.createdAt = datetime(), p.name = $name
ON MATCH SET p.seenAt = datetime()
RETURN p;

// 24. MERGE relationship
MATCH (a:Person {id: $from}), (b:Person {id: $to})
MERGE (a)-[r:KNOWS]->(b)
ON CREATE SET r.createdAt = datetime()
ON MATCH SET r.count = coalesce(r.count, 0) + 1
RETURN r;

// 25. SET labels and properties
MATCH (p:Person {id: $id})
SET p:Employee:Admin,
    p.name = 'Updated',
    p += {age: 31, city: 'Berlin'}
RETURN p;

// 26. REMOVE labels and properties
MATCH (p:Person {id: $id})
REMOVE p:Admin,
       p.temporaryProperty
RETURN p;

// 27. DELETE relationship
MATCH (:Person {id: $id})-[r:KNOWS]->()
DELETE r;

// 28. DETACH DELETE node
MATCH (p:Person {id: $id})
DETACH DELETE p;

// 29. WITH projection and filtering
MATCH (p:Person)
WITH p, p.age AS age
WHERE age > 20
RETURN p.name AS name, age;

// 30. WITH DISTINCT
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
WITH DISTINCT c
RETURN c.name;

// 31. ORDER BY, SKIP, LIMIT
MATCH (p:Person)
RETURN p.name AS name, p.age AS age
ORDER BY age DESC, name ASC
SKIP 5
LIMIT 10;

// 32. Aggregation
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
RETURN c.name AS company, count(*) AS employees, avg(p.age) AS averageAge
ORDER BY employees DESC;

// 33. Aggregation with collect
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
RETURN c.name, collect(p.name) AS names;

// 34. DISTINCT aggregate
MATCH (p:Person)
RETURN count(DISTINCT p.name) AS distinctNames;

// 35. UNWIND list
UNWIND [1, 2, 3, null] AS x
RETURN x, x * 2 AS doubled;

// 36. UNWIND nested list
UNWIND [[1, 2], [3, 4], []] AS xs
UNWIND xs AS x
RETURN x;

// 37. List indexing and slicing
WITH [10, 20, 30, 40] AS xs
RETURN xs[0] AS first, xs[-1] AS last, xs[1..3] AS middle, xs[..2] AS prefix, xs[2..] AS suffix;

// 38. List comprehension
MATCH (p:Person)
RETURN [x IN p.scores WHERE x > 10 | x * 2] AS transformed;

// 39. Pattern comprehension
MATCH (p:Person)
RETURN [(p)-[:KNOWS]->(friend) | friend.name] AS friendNames;

// 40. Map projection
MATCH (p:Person)
RETURN p {
  .name,
  .age,
  id: id(p),
  extra: 'value'
} AS projected;

// 41. Map access
WITH {name: 'Alice', nested: {x: 1}} AS m
RETURN m.name AS direct, m['name'] AS dynamic, m.nested.x AS nested;

// 42. Simple CASE
MATCH (p:Person)
RETURN CASE p.status
  WHEN 'active' THEN 1
  WHEN 'blocked' THEN -1
  ELSE 0
END AS statusCode;

// 43. Generic CASE
MATCH (p:Person)
RETURN CASE
  WHEN p.age IS NULL THEN 'unknown'
  WHEN p.age < 18 THEN 'minor'
  ELSE 'adult'
END AS ageClass;

// 44. Arithmetic precedence
RETURN 1 + 2 * 3 ^ 2 / 4 % 5 AS value;

// 45. Unary operators
RETURN -1 AS negative, +1 AS positive, NOT false AS negated;

// 46. Function calls: scalar, string, list, math, temporal
WITH '  Alice  ' AS s, [1, 2, 3] AS xs
RETURN
  trim(s) AS trimmed,
  toUpper(s) AS upper,
  size(xs) AS listSize,
  abs(-42) AS absolute,
  round(3.14159, 2) AS rounded,
  date() AS today,
  datetime() AS now;

// 47. Predicate functions
WITH [1, 2, 3, 4] AS xs
RETURN
  all(x IN xs WHERE x > 0) AS allPositive,
  any(x IN xs WHERE x = 2) AS hasTwo,
  none(x IN xs WHERE x < 0) AS noneNegative,
  single(x IN xs WHERE x = 3) AS exactlyOneThree;

// 48. reduce()
WITH [1, 2, 3, 4] AS xs
RETURN reduce(total = 0, x IN xs | total + x) AS sum;

// 49. WITH wildcard
MATCH (p:Person)
WITH *, p.name AS name
RETURN name;

// 50. UNION
MATCH (p:Person)
RETURN p.name AS name
UNION
MATCH (c:Company)
RETURN c.name AS name;

// 51. UNION ALL
MATCH (p:Person)
RETURN p.name AS name
UNION ALL
MATCH (c:Company)
RETURN c.name AS name;

// 52. CALL subquery
MATCH (p:Person)
CALL {
  WITH p
  MATCH (p)-[:KNOWS]->(f:Person)
  RETURN count(f) AS friendCount
}
RETURN p.name, friendCount;

// 53. CALL subquery with writes
MATCH (p:Person {id: $id})
CALL {
  WITH p
  CREATE (p)-[:HAS_EVENT]->(:Event {createdAt: datetime()})
  RETURN count(*) AS created
}
RETURN p, created;

// 54. EXISTS subquery expression
MATCH (p:Person)
RETURN p.name, EXISTS {
  MATCH (p)-[:KNOWS]->(:Person {name: 'Bob'})
} AS knowsBob;

// 55. COUNT subquery expression, Neo4j 5
MATCH (p:Person)
RETURN p.name, COUNT {
  MATCH (p)-[:KNOWS]->(:Person)
} AS friendCount;

// 56. COLLECT subquery expression, Neo4j 5
MATCH (p:Person)
RETURN p.name, COLLECT {
  MATCH (p)-[:KNOWS]->(f:Person)
  RETURN f.name
} AS friends;

// 57. FOREACH for conditional write trick
MATCH (p:Person {id: $id})
FOREACH (_ IN CASE WHEN p.active THEN [1] ELSE [] END |
  SET p.lastActiveAt = datetime()
)
RETURN p;

// 58. LOAD CSV
LOAD CSV WITH HEADERS FROM 'file:///people.csv' AS row
CREATE (:Person {id: row.id, name: row.name});

// 59. LOAD CSV with FIELDTERMINATOR
LOAD CSV FROM 'file:///semicolon.csv' AS row FIELDTERMINATOR ';'
RETURN row;

// 60. USE database
USE neo4j
MATCH (n)
RETURN count(n);

// 61. EXPLAIN prefix
EXPLAIN
MATCH (p:Person)
RETURN p;

// 62. PROFILE prefix
PROFILE
MATCH (p:Person)
RETURN p;

// 63. Query options
CYPHER runtime=pipelined
MATCH (p:Person)
RETURN p
LIMIT 1;

// 64. Multiple clauses mixed
MATCH (p:Person)
WHERE p.age > 20
WITH p
ORDER BY p.name
LIMIT 5
OPTIONAL MATCH (p)-[r]->(x)
RETURN p, type(r), x;

// 65. Quantified path pattern, Neo4j 5 style
MATCH p = (:Station {name: 'A'}) ((a)-[:LINK]->(b)){1,3} (:Station {name: 'B'})
RETURN p;

// 66. Quantified relationship, Neo4j 5 style
MATCH p = (:Station {name: 'A'})-[:LINK]->{1,3}(:Station {name: 'B'})
RETURN p;

// 67. Inline predicate in node pattern, Neo4j 5
MATCH (p:Person WHERE p.age > 18)-[:KNOWS]->(f:Person WHERE f.active = true)
RETURN p, f;

// 68. Inline predicate in relationship pattern, Neo4j 5
MATCH (a)-[r:KNOWS WHERE r.since > date('2020-01-01')]->(b)
RETURN a, r, b;

// 69. Shortest path family
MATCH p = shortestPath((a:Person {name: 'Alice'})-[*..5]-(b:Person {name: 'Bob'}))
RETURN p;

// 70. All shortest paths
MATCH p = allShortestPaths((a:Person {name: 'Alice'})-[*..5]-(b:Person {name: 'Bob'}))
RETURN p;

// 71. Procedure call, standalone
CALL db.labels();

// 72. Procedure call, yielded columns
CALL db.labels() YIELD label
RETURN label
ORDER BY label;

// 73. Procedure call with arguments
CALL db.propertyKeys() YIELD propertyKey
RETURN propertyKey;

// 74. Dynamic property access
MATCH (n)
WITH n, 'name' AS key
RETURN n[key] AS dynamicName;

// 75. Dynamic labels/types via expressions, Neo4j 5.26+
CREATE (n:$( $label ) {name: $name})
RETURN n;

// 76. Dynamic relationship type, Neo4j 5.26+
MATCH (a {id: $from}), (b {id: $to})
CREATE (a)-[r:$( $type )]->(b)
RETURN r;

// 77. Label expression with OR, Neo4j newer label-expression syntax
MATCH (n:Person|Company)
RETURN n;

// 78. Label expression with NOT
MATCH (n:!Deleted)
RETURN n;

// 79. Parenthesized label expression
MATCH (n:(Person|Company)&!Deleted)
RETURN n;

// 80. IS TYPED expressions, Neo4j 5
RETURN
  1 IS :: INTEGER AS isInteger,
  'x' IS :: STRING AS isString,
  null IS NOT :: BOOLEAN AS nullIsNotBoolean;

// 81. Type normalization predicate, Neo4j 5 string operator area
RETURN
  'é' IS NORMALIZED AS normalized,
  'é' IS NOT NORMALIZED AS notNormalized;

// 82. GQL-style simple match with FINISH, Neo4j 5
MATCH (n)
FINISH;

// 83. Dotted function names
RETURN foo();
RETURN foo.bar();
RETURN foo.bar.baz(1, 'x', null);
RETURN foo.bar(DISTINCT n.name);
RETURN count(*);
RETURN count(DISTINCT n.name);
RETURN n.foo AS propertyAccess;
RETURN foo.bar AS propertyAccessOnVariable;
RETURN foo.bar() AS functionCall;
