# Desktop Localization Design

## Goal

Make LinuxFS Manager approachable in the most widely useful Windows desktop
languages without changing storage, filesystem, WinFsp, mount, or read-only
behavior.

## Supported UI Languages

The first localized release ships these left-to-right user-interface locales:

| Display language | Locale |
| --- | --- |
| English | `en` |
| French | `fr-FR` |
| German | `de-DE` |
| Spanish | `es-ES` |
| Portuguese (Brazil) | `pt-BR` |
| Italian | `it-IT` |
| Polish | `pl-PL` |
| Russian | `ru-RU` |
| Simplified Chinese | `zh-CN` |
| Traditional Chinese | `zh-TW` |
| Japanese | `ja-JP` |
| Korean | `ko-KR` |

Arabic is intentionally excluded from this release because the current
desktop layout has not been designed or validated for right-to-left operation.

## Selection and Persistence

On first launch, LinuxFS Manager resolves the Windows user locale to the best
matching supported locale. If no supported match exists, it uses English.

The main-window header contains a compact language selector with **Automatic
(Windows)** followed by the twelve supported display-language names. Choosing a
specific language updates the visible application copy immediately and stores
the choice in `%APPDATA%\LinuxFS Manager\config.toml`. Choosing **Automatic**
removes the explicit preference and resolves the Windows locale again on the
next application startup.

## Translation Boundary

All LinuxFS Manager-owned copy is translated, including the main window,
About window, WinFsp prerequisite window, empty states, buttons, labels,
read-only warning, status messages, mount/unmount progress, and app-generated
errors.

The app does not translate filesystem labels, filenames, partition names,
paths, drive letters, UUIDs, or raw Windows/third-party error details. Those
values remain exact and are inserted into localized message templates.

## Architecture

Add a small typed localization module in the preview/UI layer. It defines the
supported locale enum, Windows-locale matching, selector labels, and a
complete catalog of application message keys. Each locale catalog is bundled
with the executable and validated against the English key set during tests, so
a portable build cannot omit a translation file.

The Slint declaration receives localized strings through existing/new
properties rather than interpreting storage or filesystem state. Rust formats
dynamic localized messages from typed keys and interpolation values before
setting those properties. This preserves the existing UI/core separation.

The Windows locale resolver uses standard BCP 47 locale names. It accepts
specific tags such as `pt-BR`, `zh-CN`, `zh-TW`, `ja-JP`, and `ko-KR`, then
falls back to a matching base language where appropriate.

## Configuration Compatibility

Extend the versioned `AppConfig` with an optional `ui_language` value. Missing
or malformed language settings resolve safely to **Automatic (Windows)** and
must not prevent the application from launching. Existing configuration data,
including drive-letter preference, recent images, and logging preference,
remains unchanged.

## Quality and Safety

- No Arabic or other right-to-left locale is shipped in this release.
- Localized copy must not weaken the explicit read-only warning or mask error
  categories.
- Locale switching must not rebuild, refresh, drop, mount, unmount, or alter
  any loaded source.
- Tests cover locale fallback, Windows-tag matching, catalog completeness,
  config round-trip, immediate selector updates, and safe handling of unknown
  locale values.
- Each shipped translation receives a native-speaker review before a public
  release.
