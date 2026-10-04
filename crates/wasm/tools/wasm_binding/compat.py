"""Narrow Alef 0.103.12 fixes, driven by its extracted Rust API metadata.

The pinned generator drops async on static methods and exports borrowed opaque
arguments by value. Preserve the source API's async/ownership semantics without
maintaining a second list of signatures or changing hand-written business APIs.
"""

import re


def block_end(source: str, opening: int) -> int:
    depth = 0
    tokens = re.compile(r'//[^\n]*|/\*[\s\S]*?\*/|"(?:\\.|[^"\\])*"|[{}]')
    for token in tokens.finditer(source, opening):
        if token.group() == "{":
            depth += 1
        elif token.group() == "}":
            depth -= 1
            if depth == 0:
                return token.end()
    raise ValueError("Unbalanced generated Rust block")


def preserve_source_semantics(source: str, api: dict) -> str:
    opaque = {item["name"] for item in api["types"] if item["is_opaque"]}
    edits = []
    for item in api["types"]:
        implementation = re.search(rf'^impl {re.escape(item["name"])}\s*\{{', source, re.MULTILINE)
        if implementation is None:
            raise ValueError(f"Missing generated impl: {item['name']}")
        start = implementation.end() - 1
        end = block_end(source, start)
        for method in item["methods"]:
            if method.get("binding_excluded"):
                continue
            borrowed = {param["name"]: param["ty"]["Named"] for param in method["params"]
                        if param["is_ref"] and not param["is_mut"]
                        and param["ty"].get("Named") in opaque and not param["optional"]}
            async_static = method["is_static"] and method["is_async"]
            if not borrowed and not async_static:
                continue
            pattern = rf'\bpub (?:async )?fn {re.escape(method["name"])}\s*\([\s\S]*?\)\s*->[^{{]+\{{'
            declaration = re.search(pattern, source[start:end])
            if declaration is None:
                raise ValueError(f"Missing generated method: {item['name']}.{method['name']}")
            begin = start + declaration.start()
            opening = start + declaration.end() - 1
            finish = block_end(source, opening)
            header, body = source[begin:opening], source[opening:finish]
            for name, rust_type in borrowed.items():
                pattern = rf'\b{re.escape(name)}:\s*(?:&\s*)?{re.escape(rust_type)}\b'
                header, count = re.subn(pattern, f"{name}: &{rust_type}", header)
                if count != 1:
                    raise ValueError(f"Unexpected opaque argument: {item['name']}.{method['name']}.{name}")
            if async_static:
                if "&self" in header:
                    raise ValueError("Pinned Alef changed static async generation unexpectedly")
                header = header.replace("pub fn ", "pub async fn ", 1)
                if ".await" not in body:
                    body, count = re.subn(r'\s*\.map_err\(', '.await.map_err(', body, count=1)
                    if count != 1:
                        raise ValueError("Unexpected generated async delegation")
            edits.append((begin, finish, header + body))
    for begin, end, replacement in sorted(edits, reverse=True):
        source = source[:begin] + replacement + source[end:]
    return source
