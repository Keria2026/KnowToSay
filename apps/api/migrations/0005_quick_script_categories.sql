CREATE TABLE draft_quick_categories(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE official_quick_categories(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE personal_quick_categories(id TEXT PRIMARY KEY, json TEXT NOT NULL);

INSERT INTO draft_quick_categories(id,json)
SELECT DISTINCT trim(json_extract(json,'$.category')), json_quote(trim(json_extract(json,'$.category')))
FROM draft_nodes WHERE json_extract(json,'$.kind') = 'quick'
AND trim(coalesce(json_extract(json,'$.category'),'')) NOT IN ('','未分类')
ON CONFLICT(id) DO NOTHING;
CREATE TRIGGER draft_nodes_quick_category_insert AFTER INSERT ON draft_nodes BEGIN
    INSERT INTO draft_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER draft_nodes_quick_category_update AFTER UPDATE OF json ON draft_nodes BEGIN
    INSERT INTO draft_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;

INSERT INTO official_quick_categories(id,json)
SELECT DISTINCT trim(json_extract(json,'$.category')), json_quote(trim(json_extract(json,'$.category')))
FROM official_nodes WHERE json_extract(json,'$.kind') = 'quick'
AND trim(coalesce(json_extract(json,'$.category'),'')) NOT IN ('','未分类')
ON CONFLICT(id) DO NOTHING;
CREATE TRIGGER official_nodes_quick_category_insert AFTER INSERT ON official_nodes BEGIN
    INSERT INTO official_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER official_nodes_quick_category_update AFTER UPDATE OF json ON official_nodes BEGIN
    INSERT INTO official_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;

INSERT INTO personal_quick_categories(id,json)
SELECT DISTINCT trim(json_extract(json,'$.category')), json_quote(trim(json_extract(json,'$.category')))
FROM personal_nodes WHERE json_extract(json,'$.kind') = 'quick'
AND trim(coalesce(json_extract(json,'$.category'),'')) NOT IN ('','未分类')
ON CONFLICT(id) DO NOTHING;
CREATE TRIGGER personal_nodes_quick_category_insert AFTER INSERT ON personal_nodes BEGIN
    INSERT INTO personal_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER personal_nodes_quick_category_update AFTER UPDATE OF json ON personal_nodes BEGIN
    INSERT INTO personal_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;

INSERT INTO personal_quick_categories(id,json)
SELECT DISTINCT trim(json_extract(json,'$.category')), json_quote(trim(json_extract(json,'$.category')))
FROM personal_dependencies WHERE json_extract(json,'$.kind') = 'quick'
AND trim(coalesce(json_extract(json,'$.category'),'')) NOT IN ('','未分类')
ON CONFLICT(id) DO NOTHING;
CREATE TRIGGER personal_dependencies_quick_category_insert AFTER INSERT ON personal_dependencies BEGIN
    INSERT INTO personal_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;
CREATE TRIGGER personal_dependencies_quick_category_update AFTER UPDATE OF json ON personal_dependencies BEGIN
    INSERT INTO personal_quick_categories(id,json)
    SELECT trim(json_extract(NEW.json,'$.category')), json_quote(trim(json_extract(NEW.json,'$.category')))
    WHERE json_extract(NEW.json,'$.kind') = 'quick'
    AND trim(coalesce(json_extract(NEW.json,'$.category'),'')) NOT IN ('','未分类')
    ON CONFLICT(id) DO NOTHING;
END;
