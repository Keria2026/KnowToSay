CREATE TABLE draft_flows(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE official_flows(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE personal_flows(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE personal_flow_dependencies(id TEXT PRIMARY KEY, json TEXT NOT NULL);

UPDATE draft_nodes SET json = json_set(json, '$.flowId', 'legacy-flow')
WHERE COALESCE(json_extract(json, '$.kind'), 'flow') = 'flow';
INSERT INTO draft_flows(id,json)
SELECT 'legacy-flow', json_object('id','legacy-flow','name','原有流程')
WHERE EXISTS(SELECT 1 FROM draft_nodes WHERE json_extract(json, '$.flowId') = 'legacy-flow');

UPDATE official_nodes SET json = json_set(json, '$.flowId', 'legacy-flow')
WHERE COALESCE(json_extract(json, '$.kind'), 'flow') = 'flow';
INSERT INTO official_flows(id,json)
SELECT 'legacy-flow', json_object('id','legacy-flow','name','原有流程')
WHERE EXISTS(SELECT 1 FROM official_nodes WHERE json_extract(json, '$.flowId') = 'legacy-flow');

UPDATE personal_nodes SET json = json_set(json, '$.flowId', 'legacy-flow')
WHERE COALESCE(json_extract(json, '$.kind'), 'flow') = 'flow';
INSERT INTO personal_flows(id,json)
SELECT 'legacy-flow', json_object('id','legacy-flow','name','原有流程')
WHERE EXISTS(SELECT 1 FROM personal_nodes WHERE json_extract(json, '$.flowId') = 'legacy-flow');

UPDATE personal_dependencies SET json = json_set(json, '$.flowId', 'legacy-flow')
WHERE COALESCE(json_extract(json, '$.kind'), 'flow') = 'flow';
INSERT INTO personal_flow_dependencies(id,json)
SELECT 'legacy-flow', json_object('id','legacy-flow','name','原有流程')
WHERE EXISTS(SELECT 1 FROM personal_dependencies WHERE json_extract(json, '$.flowId') = 'legacy-flow');

UPDATE releases SET json = json_set(json,
    '$.formatVersion', 2,
    '$.flows', json(CASE WHEN EXISTS (
        SELECT 1 FROM json_each(json_extract(releases.json, '$.nodes'))
        WHERE COALESCE(json_extract(value, '$.kind'), 'flow') = 'flow'
    ) THEN '[{"id":"legacy-flow","name":"原有流程"}]' ELSE '[]' END),
    '$.nodes', json(COALESCE((SELECT json_group_array(json(CASE
        WHEN COALESCE(json_extract(value, '$.kind'), 'flow') = 'flow'
        THEN json_set(value, '$.flowId', 'legacy-flow') ELSE value END))
        FROM json_each(json_extract(releases.json, '$.nodes'))), '[]')));
