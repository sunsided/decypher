// 01 Neo4j 5: quantified path pattern
MATCH p = ((a:Station)-[:LINK]->(b:Station)){1,3}
RETURN p;

// 01 openCypher: variable-length relationship
MATCH p = (a:Station)-[:LINK*1..3]->(b:Station)
RETURN p;


// 02 Neo4j 5: quantified relationship
MATCH p = (:Station {name: 'A'})-[:LINK]->{1,3}(:Station {name: 'B'})
RETURN p;

// 02 openCypher: variable-length relationship
MATCH p = (:Station {name: 'A'})-[:LINK*1..3]->(:Station {name: 'B'})
RETURN p;


// 03 Neo4j 5: inline node predicate
MATCH (p:Person WHERE p.age > 18)-[:KNOWS]->(f:Person)
RETURN p, f;

// 03 openCypher: WHERE clause predicate
MATCH (p:Person)-[:KNOWS]->(f:Person)
WHERE p.age > 18
RETURN p, f;


// 04 Neo4j 5: inline relationship predicate
MATCH (a)-[r:KNOWS WHERE r.since > date('2020-01-01')]->(b)
RETURN a, r, b;

// 04 openCypher: WHERE clause relationship predicate
MATCH (a)-[r:KNOWS]->(b)
WHERE r.since > date('2020-01-01')
RETURN a, r, b;


// 05 Neo4j 5: label OR expression
MATCH (n:Person|Company)
RETURN n;

// 05 openCypher: UNION alternative
MATCH (n:Person)
RETURN n
UNION
MATCH (n:Company)
RETURN n;


// 06 Neo4j 5: label NOT expression
MATCH (n:!Deleted)
RETURN n;

// 06 openCypher: negative label check through labels()
MATCH (n)
WHERE NOT 'Deleted' IN labels(n)
RETURN n;


// 07 Neo4j 5: parenthesized rich label expression
MATCH (n:(Person|Company)&!Deleted)
RETURN n;

// 07 openCypher: expanded predicate form
MATCH (n)
WHERE ('Person' IN labels(n) OR 'Company' IN labels(n))
  AND NOT 'Deleted' IN labels(n)
RETURN n;


// 08 Neo4j 5: dynamic node label
CREATE (n:$(label) {name: $name})
RETURN n;

// 08 openCypher: no true dynamic labels; property fallback
CREATE (n {kind: $label, name: $name})
RETURN n;


// 09 Neo4j 5: dynamic relationship type
MATCH (a {id: $from}), (b {id: $to})
CREATE (a)-[r:$(type)]->(b)
RETURN r;

// 09 openCypher: no true dynamic relationship type; fixed type fallback
MATCH (a {id: $from}), (b {id: $to})
CREATE (a)-[r:RELATED {kind: $type}]->(b)
RETURN r;


// 10 Neo4j 5: COUNT subquery expression
MATCH (p:Person)
RETURN p.name, COUNT {
  MATCH (p)-[:KNOWS]->(:Person)
} AS friendCount;

// 10 openCypher: CALL subquery equivalent-ish
MATCH (p:Person)
CALL {
  WITH p
  MATCH (p)-[:KNOWS]->(:Person)
  RETURN count(*) AS friendCount
}
RETURN p.name, friendCount;


// 11 Neo4j 5: COLLECT subquery expression
MATCH (p:Person)
RETURN p.name, COLLECT {
  MATCH (p)-[:KNOWS]->(f:Person)
  RETURN f.name
} AS friendNames;

// 11 openCypher: CALL subquery plus collect()
MATCH (p:Person)
CALL {
  WITH p
  MATCH (p)-[:KNOWS]->(f:Person)
  RETURN collect(f.name) AS friendNames
}
RETURN p.name, friendNames;


// 12 Neo4j 5: EXISTS subquery expression
MATCH (p:Person)
RETURN p.name, EXISTS {
  MATCH (p)-[:WORKS_AT]->(:Company {name: 'Neo4j'})
} AS worksAtNeo4j;

// 12 openCypher: pattern predicate style
MATCH (p:Person)
RETURN p.name, exists((p)-[:WORKS_AT]->(:Company {name: 'Neo4j'})) AS worksAtNeo4j;


// 13 Neo4j 5: FINISH
MATCH (n:Person)
SET n.seen = true
FINISH;

// 13 openCypher: update statement without RETURN
MATCH (n:Person)
SET n.seen = true;


// 14 Neo4j 5: type predicate
RETURN 1 IS :: INTEGER AS isInteger;

// 14 openCypher: no direct type predicate; function-style approximation
RETURN toInteger(1) IS NOT NULL AS isInteger;


// 15 Neo4j 5: IS NORMALIZED
RETURN 'é' IS NORMALIZED AS normalized;

// 15 openCypher: no normalization predicate; return original value
RETURN 'é' AS text;


// 16 Neo4j 5: USE database
USE neo4j
MATCH (n)
RETURN count(n);

// 16 openCypher: no database selector
MATCH (n)
RETURN count(n);


// 17 Neo4j 5: schema constraint
CREATE CONSTRAINT person_id_unique IF NOT EXISTS
FOR (p:Person)
REQUIRE p.id IS UNIQUE;

// 17 openCypher: no schema DDL equivalent
RETURN 'schema statements are outside openCypher query syntax' AS note;


// 18 Neo4j 5: index DDL
CREATE INDEX person_name_index IF NOT EXISTS
FOR (p:Person)
ON (p.name);

// 18 openCypher: no index DDL equivalent
RETURN 'index statements are outside openCypher query syntax' AS note;


// 19 Neo4j 5: richer relationship type expression in fulltext index
CREATE FULLTEXT INDEX rel_fulltext IF NOT EXISTS
FOR ()-[r:KNOWS|LIKES]-()
ON EACH [r.comment];

// 19 openCypher: no fulltext index DDL equivalent
RETURN 'fulltext index statements are Neo4j-specific' AS note;


// 20 Neo4j 5: procedure call with YIELD
CALL db.labels() YIELD label
RETURN label;

// 20 openCypher: no portable procedure namespace guarantee
RETURN 'procedure calls are implementation-specific' AS note;


// 21 Neo4j 5: query option
CYPHER runtime=pipelined
MATCH (n)
RETURN n
LIMIT 1;

// 21 openCypher: plain query without implementation option
MATCH (n)
RETURN n
LIMIT 1;


// 22 Neo4j 5: GQL-ish nested quantified pattern with inline predicate
MATCH p = ((a:Stop WHERE a.active)-[:NEXT]->(b:Stop WHERE b.active)){2,5}
RETURN p;

// 22 openCypher: variable relationship plus WHERE after MATCH
MATCH p = (a:Stop)-[:NEXT*2..5]->(b:Stop)
WHERE a.active AND b.active
RETURN p;


// 23 Neo4j 5: rich label expression with conjunction
MATCH (n:Person&Employee)
RETURN n;

// 23 openCypher: repeated labels
MATCH (n:Person:Employee)
RETURN n;


// 24 Neo4j 5: rich relationship pattern with inline predicate and variable length replacement
MATCH p = (:Person)-[r:KNOWS WHERE r.weight > 0.5]->{1,4}(:Person)
RETURN p;

// 24 openCypher: variable length, then inspect relationships
MATCH p = (:Person)-[:KNOWS*1..4]->(:Person)
WHERE all(r IN relationships(p) WHERE r.weight > 0.5)
RETURN p;


// 25 Neo4j 5: dynamic label in MATCH
MATCH (n:$(label))
RETURN n;

// 25 openCypher: label represented as data
MATCH (n)
WHERE n.kind = $label
RETURN n;
