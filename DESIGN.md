# DESIGN.md

## vantage OS — Design Philosophy

### Core goal

Build an operating system that appeals to everyone, from a 12 year old to a 70 year old, without dumbing itself down for either. The target is not "minimalism" or "beauty" alone. The target is **clarity, predictability, and low cognitive load**. Many modern desktop concepts look stunning in screenshots and become frustrating after a few months of daily use. This OS should age the opposite way: it should get more invisible, not less useful, the longer someone uses it.

The design that ages best is usually not the most futuristic one. It's the one users stop noticing because everything behaves exactly as expected.

---

### 1. Recognizable before beautiful

Users should understand the interface instantly, with zero documentation.

**Good**
- Obvious buttons
- Familiar window controls
- Visible labels
- Consistent navigation

**Bad**
- Hidden gestures
- Icon-only interfaces with no label
- Mystery meat navigation
- Excessive or surprising animations

A 70 year old should be able to open an app, change the volume, connect to Wi-Fi, and shut down the machine without asking anyone how.

---

### 2. Soft minimalism

Not extreme minimalism. Extreme minimalism hides functionality; soft minimalism organizes it.

- Keep only the important elements visible by default
- Hide advanced features one extra click away, never buried three menus deep
- Use whitespace generously
- Avoid visual clutter

Think: clean like macOS, familiar like Windows, consistent like GNOME.

**Disclosure mechanism (defined, not implied):** advanced options live behind a single, consistent pattern across the whole OS, a "More options" toggle at the bottom of any settings panel or menu. Right-click context menus are for object-specific actions only (rename, delete, properties), never a hiding place for core settings a user would need daily. No feature required for basic use is ever placed behind a right-click. This keeps "simple by default, powerful if you dig deeper" a concrete, testable rule instead of a slogan.

---

### 3. Large, forgiving touch targets (even on desktop)

- Toolbar height: 40–48px
- Interactive controls: minimum 36–44px
- Sliders with large, easy-to-grab handles
- Comfortable spacing in menus, not cramped rows

Older users and anyone with imprecise input (touch, shaky hands, trackpad) benefit enormously from this. It costs nothing for younger, more precise users.

---

### 4. Typography over graphics

A universal OS should invest more in typography than in decoration.

- One system font, high readability
- Clear hierarchy: Heading 18–20px, Body 14–16px, Labels 13–14px
- Never go below 11–12px anywhere
- Font scaling available as a top-level setting, not hidden in advanced options

---

### 5. Neutral, calm color system

Most people stare at this screen for hours a day. The interface should disappear and let content be the focus.

**Avoid:** neon colors, heavy gradients, pure black backgrounds, saturated accent colors.
**Prefer (dark mode):** dark gray (#1A1A1A), soft navy accents, muted blue highlights, off-white text.
**Prefer (light mode):** soft off-white (#F5F5F7), charcoal text (#2C2C2E), the same muted blue as the accent color. Light mode is not an inverted color scheme, it's the same calm, low-contrast philosophy at the opposite polarity. No pure white (#FFFFFF) backgrounds and no pure black (#000000) text, both are too harsh for hours of daily reading.

Where a gradient is used (e.g. the desktop background), keep it dark, subtle, and low-contrast, a mood setter, not a focal point. In light mode, the equivalent gradient runs between soft off-white and a light warm gray, same intent, same subtlety.

---

### 6. Layout

```
┌──────────────────────────────────────────────┐
│ ○ Apps          Running Apps          Time   │
└──────────────────────────────────────────────┘
```

- **Left:** system menu button (the circle). One predictable entry point for system-level actions instead of scattering controls across the screen.
- **Center:** running / pinned applications.
- **Right:** network, sound, battery, clock and date.

This layout holds up on 13" laptops, ultrawide monitors, touch devices, and for elderly users, since it never depends on screen real estate tricks or hover-only reveals.

**Overflow behavior (defined, not left to chance):** the center zone has a fixed maximum width, it never expands into the left or right zones under any circumstance. When running apps exceed that width:
1. Icons scale down toward a minimum width of 36px (still a valid touch target).
2. If they still don't fit, the oldest or least recently used icons collapse behind a "…" overflow icon at the edge of the center zone, opening a simple list on click.
3. The left menu button and right-side stats never shrink, move, or get pushed. They are fixed anchors on every screen size.

This guarantees the top bar never visually collides, even with 10+ apps open on a 13" laptop.

---

### 7. Control center (behind the circle)

```
┌──────────────────┐
│ Power            │
├──────────────────┤
│ Volume  ████░░░  │
│ Bright. █████░░  │
├──────────────────┤
│ Wi-Fi            │
│ Bluetooth        │
│ Settings         │
└──────────────────┘
```

Simple. No nested menus. No icons without a text label. Shutdown lives here, one tap from the top bar, always in the same place.

---

### 8. Window design

Most designers over-decorate windows. Don't.

- 8–12px corner radius
- Thin borders
- Slight shadow for depth, nothing heavier
- No glass effects, no excessive blur

Content should stand out more than the window frame around it.

---

### 9. Motion design

Animation exists to communicate a state change, not to impress.

**Good:** 150–200ms transitions, menu fade-ins, smooth window open/close.
**Bad:** bouncy animations, overshoot effects, long transitions.

Users care about speed more than spectacle, especially older users, for whom fast jittery motion can be disorienting rather than delightful.

---

### 10. Forgiving of mistakes

- Undo available almost everywhere
- Confirmation dialogs on destructive actions (shutdown, delete)

This isn't just an accessibility nicety, it's a safety net for everyone. Typos and misclicks happen to a teenager and a grandparent alike, just for different reasons.

---

## Where this OS comes from

This isn't a copy of any single OS. It's three specific, deliberate inheritances, not three whole philosophies stitched together. Taking everything from Windows, macOS, and Linux at once produces a confused product. Taking one clear strength from each keeps the identity coherent.

**From Windows: hardware flexibility.** The OS must run well on a wide range of hardware, from budget laptops to high-end machines, and stay backward compatible with software people already depend on. This is what lets "everyone" actually mean everyone, not just people who can afford premium hardware.

**From macOS: restraint and polish.** Fewer features shipped, but every shipped feature finished. This is the enforcement mechanism behind Section 1 and 2 above, recognizable before beautiful, soft minimalism. When in doubt, cut the feature rather than ship it half-done.

**From Linux: privacy, without the complexity.** The value taken from Linux is transparency and user trust, not open-ended configurability. In practice: no telemetry without explicit opt-in, no forced cloud account to use a local machine, no dark patterns in settings, and clear plain-language explanations of what data the OS touches. Freedom in the traditional Linux sense (terminal-first, config files, package managers as the primary interface) is explicitly out of scope. A 70 year old should never need to see a command line to use this OS fully. Anything that requires exposing that kind of complexity by default has failed Section 1.

**The rule this produces:** privacy and simplicity are not in tension here, because privacy is delivered as an invisible default, not as a control panel the user has to configure. The user gets Linux-grade respect for their data with zero Linux-grade complexity in the interface.


- Light mode
- Dark mode
- High contrast mode
- Font scaling
- Reduced motion mode
- Keyboard-first navigation
- Screen reader support

None of these should be an afterthought or a third-party plugin. They ship with the OS on day one.

---

### The cursor

The pointer follows the same philosophy as the rest of the OS: familiar shape, small deliberate refinement, not a redesign for its own sake.

- Classic arrow shape, angled like the macOS default, instantly recognizable.
- White outline around a black fill, so it stays visible on both light and dark backgrounds, including the dark gradient desktop.
- A small tail flick at the base (an extra kink in the path instead of a flat arrow close) gives it a bit of character without breaking recognizability.
- A subtle drop shadow underneath gives it depth, so it visually reads as floating above the desktop rather than sitting flat on it.

---

### What this actually is

A blend of:

- macOS typography and polish
- Windows familiarity
- GNOME simplicity
- ChromeOS approachability

The result: extremely clean, mostly flat, soft dark-gray theme, one top bar, one control center, large readable typography, very few visible controls, and no unnecessary visual effects.

**One line summary:** simple by default, powerful if you dig deeper.
