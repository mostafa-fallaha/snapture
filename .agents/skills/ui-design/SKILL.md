---
name: ui-design
description: Design and implement a distinctive, production-grade desktop UI for the Snapture tool using Rust with eframe/egui. Use this skill when building or refining UI components, layouts, overlays, or interactions for screenshot capture, annotation, or OCR workflows. Focus on creating a highly polished, memorable interface that avoids generic egui defaults and demonstrates strong visual identity.
---

This skill guides the refinement of the **Snapture** UI built with `eframe/egui`.

The goal is to make the interface feel:
- cleaner
- more modern
- more intentional
- easier to scan and interact with

But **not fancy for the sake of being fancy**.

Snapture is a utility tool. It should feel polished and pleasant, while still staying lightweight, direct, and functional.

The user may ask to improve:
- tool selection
- top action bar
- side panel layout
- color picking
- sliders and controls
- OCR/result panels
- screenshot canvas presentation
- spacing, alignment, and hierarchy

## Core Design Philosophy

Enhance the existing UI instead of reinventing it.

Prefer:
- better grouping
- better spacing
- better button styling
- clearer hierarchy
- more consistent sizing
- more attractive colors
- more natural control layouts

Avoid:
- dramatic visual themes
- overly decorative panels
- excessive floating glass effects
- dense ornamental borders
- highly experimental layouts that hurt usability

This skill should produce UI that feels like:
**“the same app, but much better designed.”**

## Design Direction

Before coding, understand the current screen and improve it with restraint:

- **Preserve workflow**: keep the UI structure familiar unless there is a strong usability reason to change it
- **Reduce clutter**: remove unnecessary visual noise
- **Improve affordances**: tools should look clickable and clearly active/inactive
- **Strengthen hierarchy**: primary actions should stand out, secondary controls should recede
- **Make controls compact and readable**: especially important for screenshot/annotation tools

## Snapture-Specific Principles

### 1. Tools should be buttons, not a scrolling list
For annotation tools like:
- Select
- Pen
- Highlight
- Rect
- Arrow
- Text
- Crop

prefer:
- compact button rows or wrapped button groups
- segmented controls
- icon + label buttons if appropriate
- clear active state

avoid:
- long plain text lists
- unnecessary scrolling for core tools
- large empty vertical menus

### 2. Color controls should feel more intentional
For color selection:
- use compact swatches
- make active color obvious
- use balanced, curated colors instead of raw defaults
- give swatches proper spacing and hover/selected feedback

The color picker should feel like a real annotation tool, not a placeholder debug control.

### 3. Sliders and numeric controls should be tidy
For thickness, text size, zoom:
- align labels and values cleanly
- keep widths consistent
- avoid awkward spacing
- show the current value clearly
- group related controls together

### 4. The canvas should remain the visual focus
The screenshot/image area is the main workspace.
UI chrome should support it, not compete with it.

Prefer:
- subtle framing
- restrained panel contrast
- enough separation to read controls clearly

Avoid:
- overpowering decorative backgrounds
- heavy styling that distracts from the captured image

### 5. Top actions should read like a real toolbar
Actions like:
- Save
- Copy
- OCR
- Undo
- Redo
- Fit

should feel like a coherent action bar:
- consistent heights
- consistent padding
- sensible grouping
- disabled states where appropriate
- stronger emphasis for the most important actions

## Visual Style Guidance

Use a restrained dark theme by default.

Target qualities:
- clean
- slightly modern
- low-noise
- practical
- sharp but not flashy

Recommended visual direction:
- dark neutral background
- subtly lighter panels
- one clear accent color
- soft borders or low-contrast strokes
- modest corner rounding
- consistent spacing system

Do not introduce:
- excessive glow
- glossy fantasy styling
- oversized decorative headers
- cinematic overlays
- overly complex layered effects

## eframe/egui Implementation Guidance

Write real working Rust code using `eframe/egui`.

Priorities:
- keep code maintainable
- encapsulate reusable UI helpers
- style through a small theme system where useful
- avoid unnecessary complexity
- fit into the existing Snapture project structure

Use idiomatic Rust patterns:
- enums for tool state
- helper functions for repeated controls
- small theme/constants section for spacing, radii, colors
- clear separation between UI state and rendering

## Recommended Improvements

Good examples of improvements this skill should make:

- replace a plain vertical tool list with compact selectable tool buttons
- improve selected/hover/disabled button states
- make color swatches look cleaner and more deliberate
- tighten sidebar spacing and grouping
- improve toolbar layout and consistency
- make sliders and labels visually aligned
- reduce the “default egui” feel without redesigning the app

## Anti-Patterns

Strictly avoid:
- redesigning the whole app into a flashy concept piece
- adding style that harms usability
- turning a utility app into a “portfolio” UI
- oversized headers and decorative sections
- unnecessary scrolling for essential tools
- excessive borders, gradients, shadows, or visual effects

## Output Expectations

When implementing UI changes:

1. Briefly state the improvement approach
2. Preserve the existing workflow unless a change clearly improves usability
3. Provide complete working Rust code using `eframe/egui`
4. Focus on practical polish over visual spectacle
5. Make the result feel more appealing, but still like Snapture

## Standard

A successful result should feel like this:

- not plain
- not ugly
- not default egui
- not overdesigned
- not flashy

It should feel like a solid desktop tool with thoughtful UI refinement.

## Important Preference for Snapture

Snapture is a practical desktop capture tool, not a design showcase.

When in doubt:
- prefer subtle improvement over dramatic restyling
- prefer compact utility controls over decorative panels
- prefer better usability over originality
- preserve the current structure and just make it cleaner

The best result is usually:
“same layout, better buttons, better spacing, better states, better colors.”