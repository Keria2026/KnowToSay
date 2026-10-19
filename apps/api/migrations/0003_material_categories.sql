CREATE TABLE draft_material_categories (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE official_material_categories (id TEXT PRIMARY KEY, json TEXT NOT NULL);

INSERT INTO draft_material_categories (id, json)
SELECT DISTINCT trim(json_extract(json, '$.category')), json_quote(trim(json_extract(json, '$.category')))
FROM draft_materials WHERE trim(coalesce(json_extract(json, '$.category'), '')) NOT IN ('', '未分类');
INSERT INTO official_material_categories (id, json)
SELECT DISTINCT trim(json_extract(json, '$.category')), json_quote(trim(json_extract(json, '$.category')))
FROM official_materials WHERE trim(coalesce(json_extract(json, '$.category'), '')) NOT IN ('', '未分类');

CREATE TRIGGER draft_material_category_insert AFTER INSERT ON draft_materials BEGIN
    INSERT INTO draft_material_categories (id, json)
    SELECT trim(json_extract(NEW.json, '$.category')), json_quote(trim(json_extract(NEW.json, '$.category')))
    WHERE trim(coalesce(json_extract(NEW.json, '$.category'), '')) NOT IN ('', '未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER draft_material_category_update AFTER UPDATE OF json ON draft_materials BEGIN
    INSERT INTO draft_material_categories (id, json)
    SELECT trim(json_extract(NEW.json, '$.category')), json_quote(trim(json_extract(NEW.json, '$.category')))
    WHERE trim(coalesce(json_extract(NEW.json, '$.category'), '')) NOT IN ('', '未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER official_material_category_insert AFTER INSERT ON official_materials BEGIN
    INSERT INTO official_material_categories (id, json)
    SELECT trim(json_extract(NEW.json, '$.category')), json_quote(trim(json_extract(NEW.json, '$.category')))
    WHERE trim(coalesce(json_extract(NEW.json, '$.category'), '')) NOT IN ('', '未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER official_material_category_update AFTER UPDATE OF json ON official_materials BEGIN
    INSERT INTO official_material_categories (id, json)
    SELECT trim(json_extract(NEW.json, '$.category')), json_quote(trim(json_extract(NEW.json, '$.category')))
    WHERE trim(coalesce(json_extract(NEW.json, '$.category'), '')) NOT IN ('', '未分类')
    ON CONFLICT(id) DO NOTHING;
END;
