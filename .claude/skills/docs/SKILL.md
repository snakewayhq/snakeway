# Skill: docs — Updating Snakeway Documentation

This skill describes how to update the Snakeway documentation site when code changes
introduce new features, settings, or behavioural changes that are not yet reflected in
the docs.

---

## Documentation Stack

| Component | Technology |
|-----------|-----------|
| Framework | [Docusaurus](https://docusaurus.io/) (classic preset, TypeScript) |
| Content | Markdown/MDX in `docs/docs/` |
| Blog | Release notes in `docs/blog/` |
| Sidebar | Manual in `docs/sidebars.ts` |
| Site config | `docs/docusaurus.config.ts` |
| Build/Preview | `just docs` (runs `npm start` in `docs/`) |
| Package manager | npm (`docs/package.json`) |

---

## Content Layout

```
docs/docs/
  introduction/          # Getting started, philosophy, roadmap
  guide/                 # How-to guides (CLI, TLS, devices, admin API, logging, static files)
  configuration/         # Configuration reference
    overview.md
    entry-point.md       # snakeway.hcl server block
    ingress.md           # Ingress/listener config
    devices/             # One file per device
      request-filter.md
      identity.md
      network-policy.md
      request-rate-limiting.md
      structured-logging.md
  internals/             # Architecture, lifecycle, mental model
docs/blog/               # Release notes as blog posts
docs/static/img/         # Images and SVG diagrams
```

---

## Frontmatter

Every doc uses minimal YAML frontmatter:

```yaml
---
title: Page Title Here
---
```

No other fields (no description, date, keywords, etc.).

---

## Writing Style and Conventions

Follow these rules precisely to match the existing documentation tone:

### Tone
- **Professional but accessible** — explain complex concepts in plain language
- **Imperative and instructional** — use "you can", "configure", "enable"
- **Concise** — dense and scannable, no verbose prose

### Structure
- Every configuration reference page follows: **Overview → Configuration Example → Sections per feature**
- Start each page with a one-line bold description of what the thing does
- Follow immediately with a `## Configuration Example` section showing a complete HCL block

### Headings
- `##` for major sections (e.g., "Method Filtering", "Body Size Limits")
- `###` for subsections (e.g., "Required Headers", "Denied Headers")
- `####` sparingly, for individual field documentation

### Code Blocks
- Use ` ```hcl ` for all HCL configuration examples
- Show a **complete, realistic example** at the top of each page
- Show **focused, minimal snippets** inline within each section
- Include comments in HCL for default values: `max_header_bytes = 16384  # 16 KB`

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
- **Bold** for field names, important concepts, and control terms
- `` `code` `` for literal field names, values, methods, and status codes
- Avoid italics — use bold instead

### Lists
- Bullet points for rules and behaviors
- Numbered lists only for ordered evaluation steps
- Nested lists for hierarchy

---

## How to Identify What Needs Updating

### For new config fields

Compare the Rust spec struct against the corresponding docs page:

| Spec file | Docs page |
|-----------|-----------|
| `crates/snakeway-conf/src/types/specification/server.rs` | `docs/docs/configuration/entry-point.md` |
| `crates/snakeway-conf/src/types/specification/ingress.rs` | `docs/docs/configuration/ingress.md` |
| `crates/snakeway-conf/src/types/specification/device/request_filter.rs` | `docs/docs/configuration/devices/request-filter.md` |
| `crates/snakeway-conf/src/types/specification/device/identity.rs` | `docs/docs/configuration/devices/identity.md` |
| `crates/snakeway-conf/src/types/specification/device/network_policy.rs` | `docs/docs/configuration/devices/network-policy.md` |
| `crates/snakeway-conf/src/types/specification/device/request_rate_limiting.rs` | `docs/docs/configuration/devices/request-rate-limiting.md` |
| `crates/snakeway-conf/src/types/specification/device/structured_logging.rs` | `docs/docs/configuration/devices/structured-logging.md` |

Every `pub` field on a spec struct should have a corresponding section or mention in the
docs page. Fields with `#[serde(default = "...")]` should show the default value in the
docs.

### For new pages

If a new feature area, device, or guide topic is added:

1. Create the markdown file under the appropriate directory
2. Add a sidebar entry in `docs/sidebars.ts` in the correct section
3. Follow the structure of an existing page in the same section as a template

### For behavioral changes

If the behavior of an existing feature changes (e.g., a new rejection reason, a new
default, a changed evaluation order), update the relevant docs page to reflect the
new behavior. Search for references to the changed behavior across all docs — it may
be mentioned in multiple places (e.g., a device doc and the lifecycle doc).

---

## Recipe: Documenting a New Config Field

### Step 1 — Identify the field and its docs page

Read the spec struct to understand:
- Field name and type
- Default value (from `#[serde(default = "...")]` or `Default` impl)
- What it controls (from the doc comment or implementation)

### Step 2 — Update the Configuration Example

Add the field to the complete HCL example at the top of the page:

```hcl
request_filter_device = {
  enable = true
  # ... existing fields ...
  client_body_timeout_seconds = 10  # NEW
}
```

### Step 3 — Add a section for the field

Add a new `##` or `###` section (match the level used by sibling fields):

```markdown
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
* Set to a lower value (e.g., 5–10 seconds) for public-facing deployments
```

### Step 4 — Verify

Preview the docs locally:

```bash
just docs
```

Open `http://localhost:4321` and navigate to the updated page.

---

## Recipe: Adding a New Docs Page

### Step 1 — Create the file

```bash
touch docs/docs/<section>/<slug>.md
```

### Step 2 — Write the frontmatter and content

```markdown
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
```

### Step 3 — Add to sidebar

Edit `docs/sidebars.ts` and add an entry in the appropriate section:

```javascript
{label: 'My New Feature', link: '/<section>/<slug>/'},
```

### Step 4 — Verify

```bash
just docs
```



