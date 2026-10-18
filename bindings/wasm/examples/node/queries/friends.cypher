// People someone knows, filtered by age.
MATCH (a:Person {name: $name})-[:KNOWS]->(b:Person)
WHERE b.age >= $minAge
RETURN b.name AS friend, b.age AS age
ORDER BY age DESC
LIMIT 10;
