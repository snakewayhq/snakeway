---
title: Writing Documentation
---

This page describes how to update this documentation site when code changes introduce new features, settings, or behavioral changes that are not yet reflected in the docs.

## Documentation Stack

| Component       | Technology                                                        |
|-----------------|-------------------------------------------------------------------|
| Framework       | [Docusaurus](https://docusaurus.io/) (classic preset, TypeScript) |
| Content         | Markdown/MDX in `docs/docs/`                                      |
| Blog            | Release notes in `docs/blog/`                                     |
| Sidebar         | Manual in `docs/sidebars.ts`                                      |
| Site config     | `docs/docusaurus.config.ts`                                       |
| Build/Preview   | `just docs` (runs `npm start` in `docs/`)                         |
| Package manager | npm (`docs/package.json`)                                         |

## Content Layout

```
docs/docs/
  introduction/          # Getting started, philosophy, roadmap
  configuration/         # Configuration reference
    overview.md
    entry-point/         # snakeway.hcl server block (index, server, tls-automation)
    ingress/             # Listener, services, routes, upstreams, static files
    devices/             # One file per device
  administration/        # CLI, TLS cert management, admin API, logging, static files
  extension/             # Device model and WASM device authoring
  contributing/          # This section
  internals/             # Architecture, lifecycle, mental model
docs/blog/               # Release notes as blog posts
docs/static/img/         # Images and SVG diagrams
```

Sections evolve, so treat this as an orientation map and check `docs/docs/` for the current structure.

## Frontmatter

Every doc uses minimal YAML frontmatter:

```yaml
---
title: Page Title Here
---
```

No other fields (no description, date, keywords, and so on).

## Writing Style and Conventions

Follow these rules precisely to match the existing documentation tone.

### Tone

- **Professional but accessible.** Explain complex concepts in plain language.
- **Imperative and instructional.** Use "you can", "configure", "enable".
- **Concise.** Dense and scannable, without verbose prose.

### Prose style

- Start each new sentence on a new line in the Markdown source.
- Do not use em dashes or any dash as punctuation.
  Restructure the sentence instead.
- Do not use semicolons in prose.
  Code examples keep their semicolons.
- Write ranges with "to", as in "1 to 5", not with a dash.
- Do not use emojis.
- Do not separate sections with `---` rules.
  Headings are sufficient.

### Structure

- Every configuration reference page follows: **Overview, then Configuration Example, then sections per feature**.
- Start each page with a one-line bold description of what the thing does.
- Follow immediately with a `## Configuration Example` section showing a complete HCL block.

### Headings

- `##` for major sections (for example "Method Filtering", "Body Size Limits").
- `###` for subsections (for example "Required Headers", "Denied Headers").
- `####` sparingly, for individual field documentation.

### Code Blocks

- Use ` ```hcl ` for all HCL configuration examples.
- Show a **complete, realistic example** at the top of each page.
- Show **focused, minimal snippets** inline within each section.
- Include comments in HCL for default values: `max_header_bytes = 16384  # 16 KB`.

### Admonitions

Use Docusaurus admonitions for callouts:

```markdown
:::note
Clarification or nuance.
:::

:::caution
Security warning or potential footgun.
:::

:::tip
Helpful advice or best practice.
:::
```

### Emphasis

- **Bold** for field names, important concepts, and control terms.
- `` `code` `` for literal field names, values, methods, and status codes.
- Avoid italics.
  Use bold instead.

### Lists

- Bullet points for rules and behaviors.
- Numbered lists only for ordered evaluation steps.
- Nested lists for hierarchy.

## How to Identify What Needs Updating

### For new config fields

Compare the Rust spec struct against the corresponding docs page.
The spec structs live under `crates/snakeway-conf/src/types/specification/`, and each family maps to a section of the configuration reference:

- **Server specs** (`specification/server/`, for example `server_spec.rs`) map to pages under `docs/docs/configuration/entry-point/`.
- **Ingress specs** (`specification/ingress/`) map to pages under `docs/docs/configuration/ingress/`.
- **Device specs** (`specification/device/`, one `*_device_spec.rs` per device) map to one page per device under `docs/docs/configuration/devices/`.

A worked example: a field added to `RequestFilterDeviceSpec` in `specification/device/request_filter_device_spec.rs` is documented in `docs/docs/configuration/devices/request-filter.md`.

Every `pub` field on a spec struct should have a corresponding section or mention in the docs page.
Fields with a declared default should show the default value in the docs.

### For new pages

If a new feature area, device, or guide topic is added:

1. Create the markdown file under the appropriate directory.
2. Add a sidebar entry in `docs/sidebars.ts` in the correct section.
3. Follow the structure of an existing page in the same section as a template.

### For behavioral changes

If the behavior of an existing feature changes (for example a new rejection reason, a new default, or a changed evaluation order), update the relevant docs page to reflect the new behavior.
Search for references to the changed behavior across all docs.
It may be mentioned in multiple places (for example a device doc and the lifecycle doc).

## Recipe: Documenting a New Config Field

### Step 1: Identify the field and its docs page

Read the spec struct to understand:

- The field name and type.
- The default value (from `#[serde(default = "...")]` or the `Default` impl).
- What it controls (from the doc comment or implementation).

### Step 2: Update the Configuration Example

Add the field to the complete HCL example at the top of the page:

```hcl
request_filter_device = {
  enable = true
  # ... existing fields ...
  client_body_timeout_seconds = 10  # NEW
}
```

### Step 3: Add a section for the field

Add a new `##` or `###` section, matching the level used by sibling fields:

````markdown
## Client Body Timeout

```hcl
client_body_timeout_seconds = 10
```

Controls how long the proxy waits for each chunk of request body data from the
client. If the client stalls mid-body for longer than this duration, the connection
is terminated.

This prevents slowloris-style attacks where an attacker sends a large
`Content-Length` but trickles body bytes to hold upstream connections.

* Default: Pingora's default (60 seconds) when not set
* Set to a lower value (for example 5 to 10 seconds) for public-facing deployments
````

### Step 4: Verify

Preview the docs locally:

```bash
just docs
```

Open `http://localhost:3000` and navigate to the updated page.

## Recipe: Adding a New Docs Page

### Step 1: Create the file

```bash
touch docs/docs/<section>/<slug>.md
```

### Step 2: Write the frontmatter and content

````markdown
---
title: My New Feature
---

The **My New Feature** does X for Y.

## Configuration Example

```hcl
my_feature = {
  enable = true
  setting = "value"
}
```

## Settings

### setting

```hcl
setting = "value"
```

Description of what it does.
````

### Step 3: Add to the sidebar

Edit `docs/sidebars.ts` and add an entry in the appropriate section:

```javascript
'<section>/<slug>',
```

### Step 4: Verify

```bash
just docs
```

## Diagrams

For flowcharts and architecture diagrams, use Mermaid.
See [Mermaid Diagrams](mermaid-diagrams.md) for the theme-correct authoring patterns.
