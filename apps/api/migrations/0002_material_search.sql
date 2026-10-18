CREATE VIRTUAL TABLE material_search USING fts5(
    material_id UNINDEXED, source UNINDEXED, name, body, notes, category, tags, cells,
    tokenize='trigram'
);
CREATE TRIGGER draft_material_insert AFTER INSERT ON draft_materials BEGIN
    INSERT INTO material_search(material_id,source,name,body,notes,category,tags,cells)
    VALUES(new.id,'draft',json_extract(new.json,'$.name'),json_extract(new.json,'$.text'),json_extract(new.json,'$.notes'),json_extract(new.json,'$.category'),json_extract(new.json,'$.tags'),json_extract(new.json,'$.rows'));
END;
CREATE TRIGGER draft_material_delete AFTER DELETE ON draft_materials BEGIN
    DELETE FROM material_search WHERE material_id=old.id AND source='draft';
END;
CREATE TRIGGER draft_material_update AFTER UPDATE ON draft_materials BEGIN
    DELETE FROM material_search WHERE material_id=old.id AND source='draft';
    INSERT INTO material_search(material_id,source,name,body,notes,category,tags,cells)
    VALUES(new.id,'draft',json_extract(new.json,'$.name'),json_extract(new.json,'$.text'),json_extract(new.json,'$.notes'),json_extract(new.json,'$.category'),json_extract(new.json,'$.tags'),json_extract(new.json,'$.rows'));
END;
CREATE TRIGGER official_material_insert AFTER INSERT ON official_materials BEGIN
    INSERT INTO material_search(material_id,source,name,body,notes,category,tags,cells)
    VALUES(new.id,'official',json_extract(new.json,'$.name'),json_extract(new.json,'$.text'),json_extract(new.json,'$.notes'),json_extract(new.json,'$.category'),json_extract(new.json,'$.tags'),json_extract(new.json,'$.rows'));
END;
CREATE TRIGGER official_material_delete AFTER DELETE ON official_materials BEGIN
    DELETE FROM material_search WHERE material_id=old.id AND source='official';
END;
CREATE TRIGGER official_material_update AFTER UPDATE ON official_materials BEGIN
    DELETE FROM material_search WHERE material_id=old.id AND source='official';
    INSERT INTO material_search(material_id,source,name,body,notes,category,tags,cells)
    VALUES(new.id,'official',json_extract(new.json,'$.name'),json_extract(new.json,'$.text'),json_extract(new.json,'$.notes'),json_extract(new.json,'$.category'),json_extract(new.json,'$.tags'),json_extract(new.json,'$.rows'));
END;
INSERT INTO material_search(material_id,source,name,body,notes,category,tags,cells)
SELECT id,'draft',json_extract(json,'$.name'),json_extract(json,'$.text'),json_extract(json,'$.notes'),json_extract(json,'$.category'),json_extract(json,'$.tags'),json_extract(json,'$.rows') FROM draft_materials;
INSERT INTO material_search(material_id,source,name,body,notes,category,tags,cells)
SELECT id,'official',json_extract(json,'$.name'),json_extract(json,'$.text'),json_extract(json,'$.notes'),json_extract(json,'$.category'),json_extract(json,'$.tags'),json_extract(json,'$.rows') FROM official_materials;
