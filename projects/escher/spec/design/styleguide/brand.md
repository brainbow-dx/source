---
title: Brand Style Guide
description: Core brand elements and usage guidelines.
type: styleguide
color-palette:
  primary: "#0042B4"
  secondary: "#FFC107"
  accent:   "#E53935"
typography:
  font-family-sans: "Inter, sans-serif"
  font-family-serif: "Merriweather, serif"
  heading-weght: 700
  body-weight: 400
---

# Brand Style Guide

Welcome to the **Escher** brand style guide. The markdown pages in this repository are parsed by our documentation harness and styled dynamically based on the YAML frontmatter above.

## Core Elements

### Color Palette

| Role      | Color  | Hex       |
|-----------|--------|----------|
| Primary   | Blue   | #0042B4 |
| Secondary | Amber  | #FFC107 |
| Accent    | Red    | #E53935 |

Use the **Primary** color for main UI elements, **Secondary** for highlights, and **Accent** sparingly to draw attention.

### Typography

* **Sans‑serif**: `Inter` – used for body text and UI.
* **Serif**: `Merriweather` – used for headings or editorial content.

#### Headings

```html
h1, h2, h3 {
    font-family: 'Merriweather', serif;
    font-weight: 700;
}
```

#### Body

```html
p, li {
    font-family: 'Inter', sans-serif;
    font-weight: 400;
}
```

## Usage Examples

### Button

```html
<button style="background-color:#0042B4;color:white;padding:.5rem 1rem;border:none;border-radius:4px;">
    Primary Action
</button>****
```

### Card

<div style="border:1px solid #e0e0e0;padding:1rem;border-radius:8px;">
	<h3>Card Title</h3>
	<p>This is an example card using the brand's color palette and typography.</p>
</div>

```html
<div style="border:1px solid #e0e0e0;padding:1rem;border-radius:8px;">
	<h3>Card Title</h3>
	<p>This is an example card using the brand's color palette and typography.</p>
</div>
```

## Further Reading

- [Typography Guidelines](../typography.md)
- [Color System](../color.md)
- [Iconography](../iconography.md)
