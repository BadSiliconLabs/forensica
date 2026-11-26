# Blockers and workarounds

| Date | Blocker | Workaround / status |
|------|---------|---------------------|
| 2026-11-27 | No `clang`, `pkg-config`, GTK3 headers; no sudo | Extracted Debian packages into `~/.local/linuxdev` (see D2). `source ~/.local/linuxdev/env.sh` before `flutter build linux`. CI installs them with apt. |
| 2026-11-27 | No Android emulator/device attached | APK is built and checked structurally; on-device testing is in the manual checklist in MORNING.md. |
