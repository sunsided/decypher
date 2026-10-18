MATCH (p:Person)-[:ACTED_IN]->(m:Movie)
WITH p, count(m) AS movies
WHERE movies > 3
RETURN p.name, m.title;
