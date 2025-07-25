---
generated_from: {{ file_path }}
last_updated: {{ last_updated }}
content_hash: {{ content_hash }}
language: {{ language }}
{% if protected_sections %}protected_sections: {{ protected_sections | json_encode }}{% endif %}
---

# {{ module_name }}

{% if file_docs %}
{{ file_docs }}
{% else %}
<!-- PROTECTED: Module Overview -->
Add a description of what this module does and why it exists.
Consider explaining the architectural decisions and design patterns used.
<!-- /PROTECTED -->
{% endif %}

## Public API

{% for module in modules %}
{% if module.visibility == "public" %}
### {{ module.name }}

{% if module.docs %}
{{ module.docs }}
{% else %}
*No documentation available*
{% endif %}

{% if module.signature %}
```{{ language }}
{{ module.signature }}
```
{% endif %}

{% if module.children %}
#### Methods

{% for child in module.children %}
{% if child.visibility == "public" %}
- **{{ child.name }}**: {% if child.docs %}{{ child.docs | truncate: 100 }}{% else %}*No description*{% endif %}
  {% endif %}
  {% endfor %}
  {% endif %}

{% endif %}
{% endfor %}

## Implementation Details

<!-- PROTECTED: Implementation Notes -->
Add notes about implementation decisions, performance considerations,
error handling strategies, or anything else that would be useful
for maintainers.
<!-- /PROTECTED -->

## Related Components

<!-- PLACEHOLDER: cross-references -->

## Testing

<!-- PROTECTED: Testing Strategy -->
Describe the testing approach for this module, including:
- Unit test coverage
- Integration test scenarios
- Mock strategies
- Performance test requirements
<!-- /PROTECTED -->

---

*This documentation was generated by Codesworth. Protected sections are preserved across regenerations.*