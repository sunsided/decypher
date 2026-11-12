// Sample queries for the playground's example picker. The first one is
// loaded on start.
export const EXAMPLES = [
  {
    name: "Recommendations",
    query: `// Products bought by people who bought what $user bought.
MATCH (u:User {id: $user})-[:BOUGHT]->(p:Product)<-[:BOUGHT]-(other:User),
      (other)-[:BOUGHT]->(rec:Product)
WHERE NOT (u)-[:BOUGHT]->(rec)
  AND rec.price <= $budget
RETURN rec.name AS product, count(DISTINCT other) AS score
ORDER BY score DESC
LIMIT $limit;`,
  },
  {
    name: "Friends of friends",
    query: `// Put the caret on a variable to see its uses; rename it under
// Symbols, explore the AST, fill in $parameters, or view the graph.
MATCH (a:Person {name: $name})-[:KNOWS*1..2]->(b:Person)
WHERE b.age >= $minAge AND NOT (b)-[:BLOCKED]->(a)
WITH b, count(*) AS paths
RETURN b.name AS friend, paths
ORDER BY paths DESC
LIMIT 10;`,
  },
  {
    name: "Subqueries",
    query: `// Variables imported into EXISTS, COUNT and CALL subqueries are
// the same variable: hover \`m\` under Symbols.
MATCH (m:Movie)
WHERE EXISTS { MATCH (m)<-[:ACTED_IN]-(:Person {born: $year}) }
CALL {
  WITH m
  MATCH (m)<-[r:REVIEWED]-(:Critic)
  RETURN avg(r.rating) AS rating
}
RETURN m.title AS title, rating,
       COUNT { (m)<-[:DIRECTED]-() } AS directors
ORDER BY rating DESC;`,
  },
  {
    name: "Import with MERGE",
    query: `// Upsert people and their employers from a parameter list.
UNWIND $rows AS row
MERGE (p:Person {email: row.email})
  ON CREATE SET p.created = timestamp()
  ON MATCH SET p.seen = timestamp()
SET p.name = row.name
MERGE (c:Company {name: row.company})
MERGE (p)-[w:WORKS_AT]->(c)
SET w.since = coalesce(w.since, row.since)
RETURN count(p) AS people;`,
  },
  {
    name: "Paths",
    query: `// The shortest route between two stations, with its total length.
MATCH (from:Station {name: $from}), (to:Station {name: $to})
MATCH route = shortestPath((from)-[:LINK*..15]-(to))
RETURN [s IN nodes(route) | s.name] AS stops,
       reduce(total = 0, l IN relationships(route) | total + l.minutes) AS minutes;`,
  },
  {
    name: "Error: out of scope",
    query: `// \`p\` is not passed on by WITH, so RETURN cannot see it.
MATCH (p:Person)-[:LIVES_IN]->(c:City)
WITH c, count(*) AS residents
RETURN c.name, residents, p.name;`,
  },
  {
    name: "Error: syntax",
    query: `// Parsing recovers after an error, so each statement reports its own.
MATCH (n:Person {name: 'Ada'}
RETURN n.name AS name;

UNWIND [1, 2, 3] AS x
RETURN x +;`,
  },
];
