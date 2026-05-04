// S01. Node uniqueness constraint
CREATE CONSTRAINT person_id_unique IF NOT EXISTS
FOR (p:Person)
REQUIRE p.id IS UNIQUE;

// S02. Node property existence constraint
CREATE CONSTRAINT person_name_exists IF NOT EXISTS
FOR (p:Person)
REQUIRE p.name IS NOT NULL;

// S03. Composite node key constraint
CREATE CONSTRAINT person_key IF NOT EXISTS
FOR (p:Person)
REQUIRE (p.country, p.id) IS NODE KEY;

// S04. Relationship property existence constraint
CREATE CONSTRAINT knows_since_exists IF NOT EXISTS
FOR ()-[r:KNOWS]-()
REQUIRE r.since IS NOT NULL;

// S05. Relationship uniqueness constraint
CREATE CONSTRAINT rel_id_unique IF NOT EXISTS
FOR ()-[r:EVENT]-()
REQUIRE r.id IS UNIQUE;

// S06. Range index
CREATE INDEX person_name_index IF NOT EXISTS
FOR (p:Person)
ON (p.name);

// S07. Composite range index
CREATE INDEX person_name_age_index IF NOT EXISTS
FOR (p:Person)
ON (p.name, p.age);

// S08. Relationship index
CREATE INDEX knows_since_index IF NOT EXISTS
FOR ()-[r:KNOWS]-()
ON (r.since);

// S09. Text index
CREATE TEXT INDEX person_bio_text IF NOT EXISTS
FOR (p:Person)
ON (p.bio);

// S10. Fulltext node index
CREATE FULLTEXT INDEX person_fulltext IF NOT EXISTS
FOR (p:Person|Employee)
ON EACH [p.name, p.bio];

// S11. Fulltext relationship index
CREATE FULLTEXT INDEX rel_fulltext IF NOT EXISTS
FOR ()-[r:KNOWS|LIKES]-()
ON EACH [r.comment];

// S12. Point index
CREATE POINT INDEX place_location_point IF NOT EXISTS
FOR (p:Place)
ON (p.location);

// S13. Lookup index for node labels
CREATE LOOKUP INDEX node_label_lookup IF NOT EXISTS
FOR (n)
ON EACH labels(n);

// S14. Lookup index for relationship types
CREATE LOOKUP INDEX rel_type_lookup IF NOT EXISTS
FOR ()-[r]-()
ON EACH type(r);

// S15. Drop index
DROP INDEX person_name_index IF EXISTS;

// S16. Drop constraint
DROP CONSTRAINT person_id_unique IF EXISTS;
