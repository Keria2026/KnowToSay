"""Generate the checked-in Excel import template without third-party dependencies."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from xml.sax.saxutils import escape
import json

headers = ['id', 'title', 'question', 'category', 'kind', 'isEntry', 'reply:zh-CN', 'reply:ko', 'reply:en', 'variables', 'branches', 'materialIds']
variables = json.dumps([{'name': '客户姓名', 'label': '客户姓名', 'required': True}], ensure_ascii=False)
branches = json.dumps([{'id': 'start-yes', 'label': '吃了', 'targetId': 'yes'}, {'id': 'start-no', 'label': '还没吃', 'targetId': 'no'}], ensure_ascii=False)
rows = [headers,
        ['start', '开场问候', '先问候客户', '入门示例', 'flow', 'true', '您好，{客户姓名}，吃饭了吗？', '{客户姓名}님, 식사하셨어요?', 'Hello {客户姓名}, have you eaten?', variables, branches, '[]'],
        ['yes', '已经用餐', '客户已经用餐', '入门示例', 'flow', 'false', '祝您今天愉快！', '좋은 하루 보내세요!', 'Have a lovely day!', '[]', '[{"id":"yes-next","label":"继续","targetId":"product"}]', '[]'],
        ['no', '尚未用餐', '客户尚未用餐', '入门示例', 'flow', 'false', '记得按时吃饭。', '식사 꼭 챙기세요.', 'Remember to eat on time.', '[]', '[{"id":"no-next","label":"继续","targetId":"product"}]', '[]'],
        ['product', '介绍产品', '两个分支汇合', '入门示例', 'flow', 'false', '我把详细资料发给您。', '자세한 자료를 보내드릴게요.', 'I will send you the details.', '[]', '[]', '[]']]

def column(index):
    result = ''
    while index:
        index, rem = divmod(index - 1, 26)
        result = chr(65 + rem) + result
    return result

sheet_rows = []
for i, row in enumerate(rows, 1):
    cells = ''.join(f'<c r="{column(j)}{i}" t="inlineStr"><is><t xml:space="preserve">{escape(str(v))}</t></is></c>' for j, v in enumerate(row, 1))
    sheet_rows.append(f'<row r="{i}">{cells}</row>')

output = Path(__file__).resolve().parents[1] / 'apps/web/static/script-template.xlsx'
with ZipFile(output, 'w', ZIP_DEFLATED) as archive:
    files = {
        '[Content_Types].xml': '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>',
        '_rels/.rels': '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        'xl/workbook.xml': '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="话术" sheetId="1" r:id="rId1"/></sheets></workbook>',
        'xl/_rels/workbook.xml.rels': '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>',
        'xl/worksheets/sheet1.xml': '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>' + ''.join(sheet_rows) + '</sheetData></worksheet>',
    }
    for name, content in files.items():
        archive.writestr(name, content)
print(output)
