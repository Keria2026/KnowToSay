CREATE TABLE draft_folders (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE official_folders (id TEXT PRIMARY KEY, json TEXT NOT NULL);

-- Assign real identities to existing categories; names are no longer identities.
INSERT INTO draft_folders(id,json)
SELECT lower(hex(randomblob(16))), json FROM draft_material_categories;
UPDATE draft_folders SET json=json_object('id',id,'name',json_extract(json,'$'),'parentId',NULL);
INSERT INTO official_folders(id,json)
SELECT coalesce((SELECT id FROM draft_folders WHERE json_extract(json,'$.name')=json_extract(c.json,'$')),lower(hex(randomblob(16)))), c.json
FROM official_material_categories c;
UPDATE official_folders SET json=json_object('id',id,'name',json_extract(json,'$'),'parentId',NULL);
UPDATE draft_materials SET json=json_set(json,'$.folderId',(SELECT id FROM draft_folders WHERE json_extract(json,'$.name')=json_extract(draft_materials.json,'$.category')))
WHERE EXISTS(SELECT 1 FROM draft_folders WHERE json_extract(json,'$.name')=json_extract(draft_materials.json,'$.category'));
UPDATE official_materials SET json=json_set(json,'$.folderId',(SELECT id FROM official_folders WHERE json_extract(json,'$.name')=json_extract(official_materials.json,'$.category')))
WHERE EXISTS(SELECT 1 FROM official_folders WHERE json_extract(json,'$.name')=json_extract(official_materials.json,'$.category'));

-- Persist the upgrade of stored release manifests so serving an older release
-- never invents new folder identities on each request.
UPDATE releases SET json=json_set(json,'$.folders',json((
  SELECT json_group_array(json_object('id',coalesce((SELECT id FROM official_folders f WHERE json_extract(f.json,'$.name')=names.name),lower(hex(randomblob(16)))),'name',names.name,'parentId',NULL))
  FROM (SELECT value AS name FROM json_each(releases.json,'$.materialCategories') UNION SELECT json_extract(value,'$.category') FROM json_each(releases.json,'$.materials') WHERE trim(coalesce(json_extract(value,'$.category'),'')) NOT IN ('','未分类')) names
))) WHERE json_type(json,'$.folders') IS NULL;
UPDATE releases SET json=json_set(json,'$.materials',json((
  SELECT json_group_array(json_set(m.value,'$.folderId',(SELECT json_extract(f.value,'$.id') FROM json_each(releases.json,'$.folders') f WHERE json_extract(f.value,'$.name')=json_extract(m.value,'$.category'))))
  FROM json_each(releases.json,'$.materials') m
)));
