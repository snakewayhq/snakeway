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

## Page Types

Every page is one of two types, detailed below.
The type determines the structure.
The prose style rules apply to both types.

### Reference pages

Reference pages document a configuration surface field by field.
They live under `configuration/`.
The reader already knows the feature exists and wants the exact fields, defaults, and behaviors.

A reference page follows this structure:

1. A one-line bold definition of what the thing is.
2. A `## Configuration Example` with a complete, realistic HCL block.
3. A field reference table.
4. One section per field or feature, each with a focused snippet.

See `configuration/devices/request-filter.md` for the model.

### Instructional guides

Instructional guides teach the reader how to accomplish a task.
They live under `introduction/`, `administration/`, and `extension/`.
The reader arrives with a problem and wants to be walked from that problem to a working result.

An instructional guide follows this structure:

1. **Introduction.** Open with the reader's situation in the first sentence, not a verdict and not a feature table.
   Name the feature that addresses the situation, then state the value it provides.
   Where a simpler alternative exists, name it and say when the reader would reach past it for this feature.
2. **Concept overview.** For any page covering more than two or three tasks, give a high-level walk-through with one complete, runnable example before the option-by-option detail.
   Close by pointing the reader to the detailed sections.
   A small single-task page may omit this.
3. **Progressive task sections.** Order sections from the simplest common task to the most advanced.
   In each section, name the situation, name the specific command or setting that addresses it, show it in use, then explain the mechanism in the order it runs.
   Lead an example with "For example" where it reads naturally.
4. **Cross-references.** Link to related guides and to the matching reference page for the exhaustive field list.

See `extension/understanding-devices.md` and `administration/admin-api.md` for the model.

### Internals pages

Internals pages under `internals/` are free-form explanatory notes about how Snakeway works.
They do not follow the instructional guide structure and read more like design notes.
Keep the formatting conventions such as heading nesting, no horizontal rules, and no emojis.
Otherwise structure them however best explains the topic.

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

- Address the reader as "you".
  Say what they can do and when they would want to do it.
- Open a section with the reader's situation, not a verdict.
  A reader who recognizes their own problem in the first sentence keeps reading.
- State one fact per sentence.
  Do not fuse a second statement onto the first with a comma and "and".
- Avoid a run of very short sentences.
  Use connective prose rather than a sequence of fragments that each land like a conclusion.
- Start each new sentence on a new line in the Markdown source.
- Do not use em dashes or any dash as punctuation.
  Restructure the sentence instead.
- Do not use semicolons in prose.
  Code examples keep their semicolons.
- Write ranges with "to", as in "1 to 5", not with a dash.
- Keep `e.g.` and `i.e.` where they read naturally.
- Do not use emojis.
- Do not separate sections with `---` rules.
  Headings are sufficient.

### Structure

Structure depends on the page type.
See [Page Types](#page-types) above.
A reference page opens with a one-line bold definition and a `## Configuration Example`.
An instructional guide opens with an Introduction that states the reader's situation.

### Headings

- `##` for major sections (for example "Method Filtering", "Body Size Limits").
- `###` for subsections (for example "Required Headers", "Denied Headers").
- `####` sparingly, for individual field documentation.
- Do not skip levels.
  A `####` sits under a `###`, which sits under a `##`.

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
