# glyphon 0.5.0 local security patch

This directory vendors `glyphon` 0.5.0 with the smallest possible change for
jcode-desktop's current `wgpu` 0.19 stack: upgrade `lru` from 0.12.1 to 0.16.3.

Reason: GitHub Dependabot reports GHSA-rhfx-m35p-ff5j for `lru < 0.16.3`.
Newer upstream `glyphon` releases also fix this, but they require much newer
`wgpu` versions and would turn this urgent security fix into a larger desktop
renderer migration. Remove this patch once jcode-desktop upgrades its GPU stack.
