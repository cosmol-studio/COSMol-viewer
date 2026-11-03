"""Render shared guides with URL-selected code and language-specific notes."""
from html import escape
import re
from docutils import nodes
from docutils.parsers.rst import Directive


def binding(directive):
    language, _, topic = directive.state.document.settings.env.docname.partition("/")
    if language not in {"python", "javascript"} or not topic:
        raise directive.error("Shared examples require a Python or JavaScript guide route")
    return language, topic


class CodeExample(Directive):
    required_arguments = 1
    has_content = True

    def run(self):
        language, topic = binding(self)
        key = self.arguments[0]
        if not re.fullmatch(r"[a-z][a-z0-9-]*", key):
            raise self.error("Example IDs must contain lowercase letters, digits and hyphens")
        content = nodes.container()
        self.state.nested_parse(self.content, self.content_offset, content)
        blocks = list(content.findall(nodes.literal_block))
        if sorted(block.get("language") for block in blocks) != ["javascript", "python"]:
            raise self.error("Each example needs exactly one Python and one JavaScript code block")
        selected = next(block for block in blocks if block["language"] == language)
        anchor = "example-" + key
        example = nodes.container(ids=[anchor], classes=["docs-code-example"])
        links = []
        for value, label in (("python", "Python"), ("javascript", "JavaScript")):
            current = ' aria-current="page"' if value == language else ""
            href = escape(f"/{value}/{topic}#{anchor}", quote=True)
            links.append(f'<a href="{href}"{current}>{label}</a>')
        example += nodes.raw("", '<nav class="docs-code-languages" aria-label="Code language">'
                             + "".join(links) + "</nav>", format="html")
        example += selected.deepcopy()
        return [example]


class LanguageOnly(Directive):
    required_arguments = 1
    has_content = True

    def run(self):
        language, _ = binding(self)
        requested = self.arguments[0]
        if requested not in {"python", "javascript"}:
            raise self.error("Expected python or javascript")
        if requested != language:
            return []
        content = nodes.container()
        self.state.nested_parse(self.content, self.content_offset, content)
        return list(content.children)


def setup(app):
    app.add_directive("code-example", CodeExample)
    app.add_directive("language-only", LanguageOnly)
    return {"parallel_read_safe": True, "parallel_write_safe": True}
