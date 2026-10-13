"""Collect registry license notices for the locked Windows dependency graph."""
import json
from pathlib import Path
root=Path(__file__).resolve().parent.parent
data=json.loads((root/'target/dependency-metadata.json').read_text(encoding='utf-8-sig'))
ids={n['id'] for n in data['resolve']['nodes']}
sections=['# Dependency notices\n\nGenerated from Cargo.lock and locally installed crate license files for the Windows build. Build and optional graph entries can appear here as well.\n']
count=0
for p in sorted(data['packages'],key=lambda p:(p['name'],p['version'])):
    if p['id'] not in ids or p['source'] is None: continue
    folder=Path(p['manifest_path']).parent
    files=sorted(f for f in folder.iterdir() if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','NOTICE','COPYRIGHT')))
    sections.append(f"\n## {p['name']} {p['version']}\n\nDeclared license: {p.get('license') or 'See crate license files'}.\n")
    if p.get('repository'):sections.append(f"\nSource: {p['repository']}\n")
    if p.get('license_file'):
        extra=folder/p['license_file']
        if extra.is_file() and extra not in files: files.append(extra)
    for f in files:
        try:text=f.read_text(encoding='utf-8')
        except (UnicodeDecodeError,OSError):continue
        sections.append(f"\n### {f.name}\n\n```text\n{text.rstrip()}\n```\n")
    count+=1
(root/'docs/DEPENDENCY_NOTICES.md').write_text(''.join(sections),encoding='utf-8')
print(f'Collected license metadata and available notices for {count} packages.')
